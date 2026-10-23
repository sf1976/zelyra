import assert from "node:assert/strict";
import {
  databaseRecordEnumOptions,
  databaseRecordInputKind,
  databaseRecordInputValue,
  parseDatabaseRecordInput,
} from "../src/database-record-inputs.js";

assert.equal(databaseRecordInputKind("BOOLEAN"), "boolean");
assert.equal(databaseRecordInputKind("TINYINT(1)"), "boolean");
assert.equal(databaseRecordInputKind("JSONB"), "json");
assert.equal(databaseRecordInputKind("DATE"), "date");
assert.equal(databaseRecordInputKind("TIME"), "time");
assert.equal(databaseRecordInputKind("TIME(6)"), "time");
assert.equal(databaseRecordInputKind("TIME WITH TIME ZONE"), "text");
assert.equal(databaseRecordInputKind("DECIMAL(10,2)"), "number");
assert.deepEqual(databaseRecordEnumOptions("ENUM('open','in progress','it''s ready')"), ["open", "in progress", "it's ready"]);
assert.deepEqual(databaseRecordEnumOptions("ENUM('active','owner\\'s')"), ["active", "owner's"]);
assert.deepEqual(databaseRecordEnumOptions("ENUM('incomplete', broken)"), []);
assert.equal(databaseRecordInputValue("boolean", "0"), false);
assert.equal(databaseRecordInputValue("boolean", "true"), true);
assert.deepEqual(databaseRecordInputValue("json", '{"enabled":true}'), '{\n  "enabled": true\n}');
assert.equal(parseDatabaseRecordInput("boolean", "false"), false);
assert.deepEqual(parseDatabaseRecordInput("json", '{"count":2}'), { count: 2 });
assert.throws(() => parseDatabaseRecordInput("json", "{"), SyntaxError);
console.log("Studio database record inputs match SQL column types");
