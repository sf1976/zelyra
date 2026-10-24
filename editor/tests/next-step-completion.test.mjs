import assert from "node:assert/strict";
import { isTopLevelBlankPosition, nextCrudCompletions, nextFormCompletions } from "../src/next-step-completion.js";

const topLevelSource = "table customers { id: Id primary auto }\n\nfn main() {}";
const nextStepPosition = topLevelSource.indexOf("\n\n") + 1;
assert.equal(isTopLevelBlankPosition(topLevelSource, nextStepPosition), true);
assert.equal(isTopLevelBlankPosition("fn main() {\n  \n}", 13), false);
assert.equal(isTopLevelBlankPosition("fn main() {}\ncrud", 17), false);
assert.equal(isTopLevelBlankPosition("## { ignored } ##\n\n", 19), true);

const map = {
  incomplete: false,
  nodes: [
    { id: "table:src/customers.zyl:customers", kind: "table", label: "customers" },
    { id: "table:src/orders.zyl:orders", kind: "table", label: "orders" },
    { id: "table:src/archive.zyl:customers", kind: "table", label: "archive_customers" },
  ],
  edges: [{ from: "crud:main.zyl:Order", to: "table:src/orders.zyl:orders", label: "uses table" }],
};
assert.deepEqual(nextCrudCompletions(map, "de"), [{
  label: "crud Customers -> customers {}",
  apply: "crud Customers -> customers {}",
  type: "keyword",
  detail: "CRUD-Block für die Tabelle customers",
}, {
  label: "crud ArchiveCustomers -> archive_customers {}",
  apply: "crud ArchiveCustomers -> archive_customers {}",
  type: "keyword",
  detail: "CRUD-Block für die Tabelle archive_customers",
}]);
assert.deepEqual(nextCrudCompletions({ ...map, incomplete: true }, "de"), []);
assert.deepEqual(nextCrudCompletions({ ...map, edges: [] }, "en").map((item) => item.label), [
  "crud Customers -> customers {}",
  "crud Orders -> orders {}",
  "crud ArchiveCustomers -> archive_customers {}",
]);
assert.deepEqual(nextCrudCompletions({ ...map, nodes: [...map.nodes, { ...map.nodes[0], id: "table:duplicate.zyl:customers" }] }, "de").map((item) => item.label), [
  "crud ArchiveCustomers -> archive_customers {}",
]);
const caseCollidingTables = {
  ...map,
  nodes: [...map.nodes, { id: "table:src/upper.zyl:Customers", kind: "table", label: "Customers" }],
  edges: [],
};
assert.deepEqual(nextCrudCompletions(caseCollidingTables, "en").map((item) => item.label), [
  "crud Orders -> orders {}",
  "crud ArchiveCustomers -> archive_customers {}",
]);
assert.deepEqual(nextCrudCompletions({ ...map, nodes: [...map.nodes, { id: "form:main.zyl:Customers", kind: "form", label: "Customers" }] }, "en").map((item) => item.label), [
  "crud ArchiveCustomers -> archive_customers {}",
]);
const formMap = {
  ...map,
  nodes: map.nodes.map((node) => node.kind === "table" && node.label === "customers"
    ? { ...node, editable_fields: ["name", "email"] }
    : node.kind === "table" && node.label === "orders"
      ? { ...node, editable_fields: ["number"] }
      : node.kind === "table"
        ? { ...node, editable_fields: [] }
        : node),
  edges: [...map.edges, { from: "form:main.zyl:OrderForm", to: "table:src/orders.zyl:orders", label: "writes table" }],
};
assert.deepEqual(nextFormCompletions(formMap, "de"), [{
  label: "form CustomersForm -> customers { … }",
  apply: "form CustomersForm -> customers {\n    fields {\n        name\n        email\n    }\n}",
  type: "keyword",
  detail: "Formulargerüst mit editierbaren Feldern aus customers",
}]);
assert.deepEqual(nextFormCompletions({ ...formMap, incomplete: true }, "de"), []);
assert.deepEqual(nextFormCompletions({ ...formMap, nodes: formMap.nodes.map((node) => node.label === "customers" ? { ...node, editable_fields: [] } : node) }, "en"), []);
assert.deepEqual(nextFormCompletions({ ...formMap, nodes: [...formMap.nodes, { id: "form:other.zyl:CustomersForm", kind: "form", label: "CustomersForm" }] }, "en"), []);
assert.deepEqual(nextFormCompletions({
  ...formMap,
  nodes: [...formMap.nodes, { id: "table:src/upper.zyl:Customers", kind: "table", label: "Customers", editable_fields: ["title"] }],
  edges: formMap.edges,
}, "en"), []);
console.log("Studio next-step completion unit tests passed");
