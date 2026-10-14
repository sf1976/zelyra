-- Repeatable tutorial data. The stable email, SKU, and invoice number keep
-- later `zelyra setup --all` and Studio Run calls from adding duplicates.
INSERT INTO customers (name, email)
SELECT 'Beispiel GmbH', 'kontakt@beispiel.test'
WHERE NOT EXISTS (
    SELECT 1 FROM customers WHERE email = 'kontakt@beispiel.test'
);

INSERT INTO customers (name, email)
SELECT 'Nordlicht Büro', 'rechnung@nordlicht.test'
WHERE NOT EXISTS (
    SELECT 1 FROM customers WHERE email = 'rechnung@nordlicht.test'
);

INSERT INTO items (sku, name, unit_price, active)
SELECT 'ZYL-BERATUNG', 'Einrichtung und Beratung', 125.00, TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM items WHERE sku = 'ZYL-BERATUNG'
);

INSERT INTO items (sku, name, unit_price, active)
SELECT 'ZYL-WARTUNG', 'Monatliche Wartung', 48.00, TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM items WHERE sku = 'ZYL-WARTUNG'
);

INSERT INTO items (sku, name, unit_price, active)
SELECT 'ZYL-PAKET', 'Arbeitsplatzpaket', 89.90, TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM items WHERE sku = 'ZYL-PAKET'
);

INSERT INTO invoices (invoice_number, customer_id, issued_at, status)
SELECT 'RE-2026-001', customers.id, '2026-01-15', 'draft'
FROM customers
WHERE customers.email = 'kontakt@beispiel.test'
  AND NOT EXISTS (
      SELECT 1 FROM invoices WHERE invoice_number = 'RE-2026-001'
  );

INSERT INTO invoice_lines (invoice_id, item_id, quantity, unit_price)
SELECT invoices.id, items.id, 2, items.unit_price
FROM invoices
JOIN items ON items.sku = 'ZYL-BERATUNG'
WHERE invoices.invoice_number = 'RE-2026-001'
  AND NOT EXISTS (
      SELECT 1 FROM invoice_lines
      WHERE invoice_id = invoices.id AND item_id = items.id
  );

INSERT INTO invoice_lines (invoice_id, item_id, quantity, unit_price)
SELECT invoices.id, items.id, 1, items.unit_price
FROM invoices
JOIN items ON items.sku = 'ZYL-WARTUNG'
WHERE invoices.invoice_number = 'RE-2026-001'
  AND NOT EXISTS (
      SELECT 1 FROM invoice_lines
      WHERE invoice_id = invoices.id AND item_id = items.id
  );
