//! Typed EN 16931 / Factur-X / XRechnung CrossIndustryInvoice XML generation
//! and XXE-safe validation without external XML dependencies.

use zelyra_ast::decimal;

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_money_2dp(scaled: i128) -> String {
    let negative = scaled < 0;
    let abs = scaled.unsigned_abs();
    let scale = 10_u128.pow(decimal::SCALE_DIGITS);
    let whole = abs / scale;
    let frac_9 = abs % scale;
    // Round 9-digit scaled decimal to 2 decimal places (half-even)
    let divisor: u128 = 10_000_000;
    let mut cents = frac_9 / divisor;
    let rem = frac_9 % divisor;
    let twice = rem * 2;
    if twice > divisor || (twice == divisor && cents % 2 != 0) {
        cents += 1;
    }
    let (final_whole, final_cents) = if cents >= 100 {
        (whole + 1, cents - 100)
    } else {
        (whole, cents)
    };
    if negative {
        format!("-{final_whole}.{final_cents:02}")
    } else {
        format!("{final_whole}.{final_cents:02}")
    }
}

fn is_valid_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let year: u32 = match date[0..4].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let month: u32 = match date[5..7].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let day: u32 = match date[8..10].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    (1900..=9999).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn is_valid_iso_currency(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_en16931_xml(
    invoice_number: &str,
    issue_date: &str,
    currency: &str,
    seller_name: &str,
    buyer_name: &str,
    net_total: i128,
    tax_total: i128,
    gross_total: i128,
) -> Result<String, String> {
    if invoice_number.trim().is_empty() {
        return Err("einvoice_xml: invoice_number must not be empty".into());
    }
    if !is_valid_iso_date(issue_date) {
        return Err("einvoice_xml: issue_date must be YYYY-MM-DD".into());
    }
    if !is_valid_iso_currency(currency) {
        return Err(
            "einvoice_xml: currency must be a 3-letter uppercase ISO-4217 code (e.g. EUR)".into(),
        );
    }
    if seller_name.trim().is_empty() || buyer_name.trim().is_empty() {
        return Err("einvoice_xml: seller_name and buyer_name must not be empty".into());
    }
    if net_total < 0 || tax_total < 0 || gross_total < 0 {
        return Err("einvoice_xml: invoice totals must be non-negative".into());
    }
    let expected_gross = net_total
        .checked_add(tax_total)
        .ok_or_else(|| "einvoice_xml: arithmetic overflow computing gross total".to_owned())?;
    if expected_gross != gross_total {
        return Err(format!(
            "einvoice_xml: arithmetic mismatch: net ({}) + tax ({}) != gross ({})",
            format_money_2dp(net_total),
            format_money_2dp(tax_total),
            format_money_2dp(gross_total)
        ));
    }

    let cii_date = issue_date.replace('-', "");
    let net_str = format_money_2dp(net_total);
    let tax_str = format_money_2dp(tax_total);
    let gross_str = format_money_2dp(gross_total);

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100" xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100">
  <rsm:ExchangedDocumentContext>
    <ram:GuidelineSpecifiedDocumentContextParameter>
      <ram:ID>urn:cen.eu:en16931:2017</ram:ID>
    </ram:GuidelineSpecifiedDocumentContextParameter>
  </rsm:ExchangedDocumentContext>
  <rsm:ExchangedDocument>
    <ram:ID>{inv}</ram:ID>
    <ram:TypeCode>380</ram:TypeCode>
    <ram:IssueDateTime>
      <udt:DateTimeString format="102">{date}</udt:DateTimeString>
    </ram:IssueDateTime>
  </rsm:ExchangedDocument>
  <rsm:SupplyChainTradeTransaction>
    <ram:ApplicableHeaderTradeAgreement>
      <ram:SellerTradeParty>
        <ram:Name>{seller}</ram:Name>
      </ram:SellerTradeParty>
      <ram:BuyerTradeParty>
        <ram:Name>{buyer}</ram:Name>
      </ram:BuyerTradeParty>
    </ram:ApplicableHeaderTradeAgreement>
    <ram:ApplicableHeaderTradeSettlement>
      <ram:InvoiceCurrencyCode>{curr}</ram:InvoiceCurrencyCode>
      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>
        <ram:TaxBasisTotalAmount>{net}</ram:TaxBasisTotalAmount>
        <ram:TaxTotalAmount currencyID="{curr}">{tax}</ram:TaxTotalAmount>
        <ram:GrandTotalAmount>{gross}</ram:GrandTotalAmount>
        <ram:DuePayableAmount>{gross}</ram:DuePayableAmount>
      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>
    </ram:ApplicableHeaderTradeSettlement>
  </rsm:SupplyChainTradeTransaction>
</rsm:CrossIndustryInvoice>"#,
        inv = xml_escape(invoice_number.trim()),
        date = cii_date,
        seller = xml_escape(seller_name.trim()),
        buyer = xml_escape(buyer_name.trim()),
        curr = currency,
        net = net_str,
        tax = tax_str,
        gross = gross_str,
    ))
}

fn extract_tag_text<'a>(xml: &'a str, open_prefix: &str, close_tag: &str) -> Option<&'a str> {
    let start_pos = xml.find(open_prefix)?;
    let after_open = &xml[start_pos + open_prefix.len()..];
    let gt = after_open.find('>')?;
    let content = &after_open[gt + 1..];
    let end_pos = content.find(close_tag)?;
    Some(content[..end_pos].trim())
}

pub fn validate_en16931_xml(xml: &str) -> Result<bool, String> {
    let upper = xml.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err("einvoice_validate: XML DOCTYPE/ENTITY declarations are forbidden (XXE protection)".into());
    }
    if !xml.contains("<rsm:CrossIndustryInvoice") || !xml.contains("</rsm:CrossIndustryInvoice>") {
        return Err("einvoice_validate: missing CrossIndustryInvoice root element".into());
    }
    if !xml.contains("urn:cen.eu:en16931:2017") {
        return Err("einvoice_validate: missing EN 16931 profile URN".into());
    }
    for (tag, label) in [
        ("<ram:SellerTradeParty>", "SellerTradeParty"),
        ("<ram:BuyerTradeParty>", "BuyerTradeParty"),
        ("<ram:InvoiceCurrencyCode", "InvoiceCurrencyCode"),
    ] {
        if !xml.contains(tag) {
            return Err(format!("einvoice_validate: missing required element `{label}`"));
        }
    }
    let net_str = extract_tag_text(xml, "<ram:TaxBasisTotalAmount", "</ram:TaxBasisTotalAmount>")
        .ok_or_else(|| "einvoice_validate: missing TaxBasisTotalAmount".to_owned())?;
    let tax_str = extract_tag_text(xml, "<ram:TaxTotalAmount", "</ram:TaxTotalAmount>")
        .ok_or_else(|| "einvoice_validate: missing TaxTotalAmount".to_owned())?;
    let gross_str = extract_tag_text(xml, "<ram:GrandTotalAmount", "</ram:GrandTotalAmount>")
        .ok_or_else(|| "einvoice_validate: missing GrandTotalAmount".to_owned())?;

    let net = decimal::parse_scaled(net_str, decimal::SCALE_DIGITS)
        .ok_or_else(|| format!("einvoice_validate: invalid TaxBasisTotalAmount `{net_str}`"))?;
    let tax = decimal::parse_scaled(tax_str, decimal::SCALE_DIGITS)
        .ok_or_else(|| format!("einvoice_validate: invalid TaxTotalAmount `{tax_str}`"))?;
    let gross = decimal::parse_scaled(gross_str, decimal::SCALE_DIGITS)
        .ok_or_else(|| format!("einvoice_validate: invalid GrandTotalAmount `{gross_str}`"))?;
    let expected = net
        .checked_add(tax)
        .ok_or_else(|| "einvoice_validate: arithmetic overflow".to_owned())?;
    if expected != gross {
        return Err(format!(
            "einvoice_validate: TaxBasisTotalAmount ({net_str}) + TaxTotalAmount ({tax_str}) != GrandTotalAmount ({gross_str})"
        ));
    }
    Ok(true)
}
