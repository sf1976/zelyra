import assert from "node:assert/strict";
import { sqlBodyPrefix, sqlCompletionOptions } from "../src/sql-completion.js";

const source = [
  'let example = "sql<Row[]> { SELECT fake FROM nowhere }"',
  "# sql<Row[]> { SELECT commented FROM nowhere }",
  "tableview Rows { source sql<Row[]> { SELECT id, name FROM customers WHERE name = 'sql { fake }' } }",
].join("\n");
const queryStart = source.indexOf("SELECT id");
assert.equal(sqlBodyPrefix(source, queryStart + "SELECT id, name FROM customers".length), " SELECT id, name FROM customers");
assert.equal(sqlBodyPrefix(source, source.indexOf("example") + 5), null, "SQL-like text in strings must not become an SQL block");
assert.equal(sqlBodyPrefix(source, source.indexOf("commented") + 4), null, "SQL-like text in comments must not become an SQL block");

const schema = { nodes: [
  { kind: "table", label: "customers", columns: ["id", "name", "email"] },
  { kind: "table", label: "orders", columns: ["id", "customer_id", "total"] },
  { kind: "table", label: "ambiguous", columns: ["hidden"] },
  { kind: "table", label: "ambiguous", columns: ["also_hidden"] },
] };
const tableSuggestions = sqlCompletionOptions("SELECT * FROM ", schema, "table").map((item) => item.label);
assert.deepEqual(tableSuggestions.sort(), ["customers", "orders"]);

const qualified = sqlCompletionOptions("SELECT c.name FROM customers AS c WHERE c.", schema);
assert.deepEqual(qualified.map((item) => item.label).sort(), ["email", "id", "name"]);
assert.ok(qualified.every((item) => item.detail.startsWith("customers.")));

const unqualified = sqlCompletionOptions("SELECT name FROM customers JOIN orders ON customers.id = orders.customer_id WHERE ", schema);
assert.deepEqual(unqualified.map((item) => item.label).sort(), ["customer_id", "email", "name", "total"]);
assert.equal(sqlCompletionOptions("SELECT ", schema).length, 0, "unresolved tables must not produce guessed fields");

console.log("Zelyra SQL completion resolves declared tables and unambiguous columns.");
