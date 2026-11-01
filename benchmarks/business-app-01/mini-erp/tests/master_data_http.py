#!/usr/bin/env python3
"""Black-box HTTP regression for the native Zelyra master-data CRUD flow.

The application and all business logic under test run in Zelyra. This script
only drives the browser-facing HTTP interface against a disposable database.
"""

from __future__ import annotations

import http.cookiejar
import os
import re
import time
import urllib.error
import urllib.parse
import urllib.request


BASE_URL = os.environ.get("ZELYRA_BASE_URL", "http://127.0.0.1:4315").rstrip("/")
EMAIL = os.environ["MINI_ERP_ADMIN_EMAIL"]
PASSWORD = os.environ["MINI_ERP_ADMIN_PASSWORD"]
SUFFIX = str(time.time_ns())[-6:]

cookies = http.cookiejar.CookieJar()
http_client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookies))


def get(path: str) -> str:
    with http_client.open(f"{BASE_URL}{path}") as response:
        return response.read().decode("utf-8")


def post(path: str, fields: dict[str, str]) -> tuple[str, str]:
    request = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=urllib.parse.urlencode(fields).encode("utf-8"),
        headers={"Origin": BASE_URL},
    )
    try:
        with http_client.open(request) as response:
            return response.geturl(), response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise AssertionError(f"POST {path} returned HTTP {error.code}: {body[:600]}") from error


def csrf_token(page: str) -> str:
    match = re.search(r'name="_zelyra_csrf" value="([^"]+)"', page)
    assert match, "form did not contain the native CSRF field"
    return match.group(1)


def relation_options(page: str, field: str) -> list[tuple[str, str]]:
    match = re.search(
        rf'<select\b[^>]*name="{re.escape(field)}"[^>]*>(.*?)</select>',
        page,
        re.DOTALL,
    )
    assert match, f"relation field {field!r} was not rendered as a select"
    return [
        (value, re.sub(r"<[^>]*>", "", label).strip())
        for value, label in re.findall(
            r'<option\s+value="([^"]*)"[^>]*>(.*?)</option>', match.group(1), re.DOTALL
        )
    ]


def create(path: str, fields: dict[str, str]) -> str:
    page = get(path)
    fields = {"_zelyra_csrf": csrf_token(page), **fields}
    final_url, result = post(path, fields)
    assert "z-error-summary" not in result, f"create form rejected values at {path}"
    assert "zelyra_success" in final_url, f"create form did not finish successfully: {final_url}"
    return result


login = get("/login")
post(
    "/login",
    {
        "_zelyra_csrf": csrf_token(login),
        "email": EMAIL,
        "password": PASSWORD,
    },
)

group_code = f"E2E-{SUFFIX}"
group_name = f"Benchmark group {SUFFIX}"
group_page = get("/article_groups/new")
active_checkbox = re.search(r'<input\b[^>]*name="active"[^>]*>', group_page)
assert active_checkbox and "checked" in active_checkbox.group(), "Bool default true must render checked"
create(
    "/article_groups/new",
    {
        "code": group_code,
        "name": group_name,
        "description": "Created by the master-data HTTP regression",
        "active": "true",
    },
)
group_search = get(f"/article_groups?search={urllib.parse.quote(group_name)}")
assert group_name in group_search, "generated CRUD search did not find the group"
assert group_name in get("/article_groups?filter_active=true"), "active filter omitted the active group"
assert group_name not in get("/article_groups?filter_active=false"), "inactive filter included the active group"

unit_code = f"U{SUFFIX}"
unit_label = f"Benchmark unit {SUFFIX}"
create(
    "/units/new",
    {
        "code": unit_code,
        "label": unit_label,
        "quantity_scale": "0",
        "active": "true",
    },
)

article_page = get("/articles/new")
groups = relation_options(article_page, "group")
units = relation_options(article_page, "unit")
assert any(label == group_name for _, label in groups), "active article group missing from relation options"
assert any(label == unit_label for _, label in units), "unit label missing from relation options"
group_id = next(value for value, label in groups if label == group_name)

article_number = f"E2E-{SUFFIX}"
final_url, result = post(
    "/articles/new",
    {
        "_zelyra_csrf": csrf_token(article_page),
        "article_number": article_number,
        "name": f"Benchmark article {SUFFIX}",
        "group": next(value for value, label in groups if label == group_name),
        "unit": next(value for value, label in units if label == unit_label),
        "purchase_price": "5.50",
        "sales_price": "9.90",
        "tax_rate": "19.00",
        "minimum_stock": "2",
        "reorder_point": "4",
        "active": "true",
    },
)
assert "z-error-summary" not in result, "article form rejected valid relation IDs"
assert "zelyra_success" in final_url, f"article create did not finish successfully: {final_url}"
assert article_number in get("/articles"), "created article missing from generated article list"

archive_page = get(f"/article_groups/{group_id}")
archive_path = f"/article_groups/{group_id}/delete"
archive_url, _ = post(archive_path, {"_zelyra_csrf": csrf_token(archive_page)})
assert "zelyra_success" in archive_url, "group archive did not finish successfully"
remaining_groups = relation_options(get("/articles/new"), "group")
assert not any(label == group_name for _, label in remaining_groups), "archived group remains selectable"
assert group_name not in get("/article_groups"), "archived group remains in the default list"
assert article_number in get("/articles"), "archiving a group hid its historical article"

print(f"PASS: {article_number} linked to {group_name} and {unit_label}; archived group excluded")
