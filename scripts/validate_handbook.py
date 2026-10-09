#!/usr/bin/env python3
"""
Zelyra Handbook Code Snippet Validator
Validates Zelyra code blocks across the split German and English handbook chapters
"""

import sys
import re
from pathlib import Path

def validate_snippet(code, line_offset=0):
    stack = []
    lines = code.splitlines()
    in_block_comment = False
    
    for idx, line in enumerate(lines):
        line_num = idx + 1 + line_offset
        j = 0
        in_str = False
        str_char = ''
        
        while j < len(line):
            c = line[j]
            next_c = line[j+1] if j + 1 < len(line) else ''
            
            if in_block_comment:
                if c == '*' and next_c == '/':
                    in_block_comment = False
                    j += 2
                    continue
                j += 1
                continue
                
            if in_str:
                if c == '\\':
                    j += 2
                    continue
                if c == str_char:
                    in_str = False
                j += 1
                continue
                
            if c == '/' and next_c == '/':
                break
            if c == '/' and next_c == '*':
                in_block_comment = True
                j += 2
                continue
                
            if c in ('"', "'"):
                in_str = True
                str_char = c
                j += 1
                continue
                
            if c in '{[(':
                stack.append((c, line_num))
            elif c in '}])':
                if not stack:
                    return False, f"Unexpected closing '{c}' at line {line_num}"
                last_c, last_line = stack.pop()
                expected = {'{': '}', '[': ']', '(': ')'}[last_c]
                if c != expected:
                    return False, f"Mismatched delimiter: expected '{expected}' for '{last_c}' from line {last_line}, got '{c}' at line {line_num}"
            j += 1
            
    if stack:
        last_c, last_line = stack[-1]
        return False, f"Unclosed '{last_c}' opened at line {last_line}"
        
    return True, None

def check_file(path):
    p = Path(path)
    if not p.exists():
        return 0, [f"File not found: {path}"]
        
    text = p.read_text(encoding="utf-8")
    lines = text.splitlines()
    snippets = []
    in_block = False
    curr_lines = []
    start_line = 0
    chapter = "Start"
    
    for i, line in enumerate(lines):
        line_num = i + 1
        m_chap = re.match(r"^#{1,3}\s+(.+)$", line)
        if m_chap:
            chapter = m_chap.group(1).strip()
            
        if re.match(r"^```(?:zelyra|zyl)\s*$", line, re.IGNORECASE):
            in_block = True
            start_line = line_num
            curr_lines = []
            continue
            
        if in_block and re.match(r"^```\s*$", line):
            in_block = False
            snippets.append({
                "code": "\n".join(curr_lines),
                "line": start_line,
                "chapter": chapter
            })
            curr_lines = []
            continue
            
        if in_block:
            curr_lines.append(line)
            
    errors = []
    for snip in snippets:
        ok, err = validate_snippet(snip["code"], snip["line"])
        if not ok:
            errors.append(f"Line {snip['line']} ({snip['chapter']}): {err}")
            
    return len(snippets), errors

def main():
    root = Path(__file__).resolve().parent.parent
    files = {
        "German Handbook": [
            root / "docs/handbook/de/handbuch.md",
            root / "docs/handbook/de/lehrbuch-grundlagen.md",
            root / "docs/handbook/de/lehrbuch-anwendungen.md",
            root / "docs/handbook/de/technisches-referenzhandbuch.md",
            root / "docs/handbook/de/anhaenge.md",
        ],
        "English Handbook": [
            root / "docs/handbook/en/handbook.md",
            root / "docs/handbook/en/textbook-foundations.md",
            root / "docs/handbook/en/textbook-applications.md",
            root / "docs/handbook/en/technical-reference.md",
            root / "docs/handbook/en/appendices.md",
        ],
    }
    
    total = 0
    all_errors = []
    print("=" * 60)
    print(" ZELYRA HANDBOOK CODE SNIPPET VALIDATION")
    print("=" * 60)
    
    for name, paths in files.items():
        count = 0
        errors = []
        for fpath in paths:
            file_count, file_errors = check_file(fpath)
            count += file_count
            errors.extend(f"{fpath.relative_to(root)} -> {error}" for error in file_errors)
        total += count
        status = "✓ OK" if not errors else f"✕ {len(errors)} errors"
        print(f"[{status}] {name}: {count} snippets checked across {len(paths)} files")
        for error in errors:
            all_errors.append(error)
            print(f"       ERROR: {error}")
            
    print("-" * 60)
    if all_errors:
        print(f"FAILED: Found {len(all_errors)} issues across {total} snippets.")
        sys.exit(1)
    else:
        print(f"SUCCESS: All {total} handbook code examples verified successfully.")
        sys.exit(0)

if __name__ == "__main__":
    main()
