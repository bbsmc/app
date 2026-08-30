import assert from "node:assert/strict";
import test from "node:test";

import { formatMoney, subtractMoney } from "../src/utils/format-money.js";

test("formatMoney preserves exact cents without IEEE-754 underflow", () => {
  assert.equal(formatMoney("5.02"), "¥5.02");
  assert.equal(formatMoney("8.03"), "¥8.03");
  assert.equal(formatMoney("0.29"), "¥0.29");
  assert.equal(formatMoney(5.02), "¥5.02");
});

test("formatMoney handles negatives while preserving floor-to-cents semantics", () => {
  assert.equal(formatMoney("-5.02"), "¥-5.02");
  assert.equal(formatMoney("-5.029"), "¥-5.03");
  assert.equal(formatMoney("-10000", true), "¥-10,000.00");
});

test("formatMoney formats large exact decimal strings and abbreviations", () => {
  assert.equal(formatMoney("123456789012345.67"), "¥123,456,789,012,345.67");
  assert.equal(formatMoney("12550", true), "¥1.26万");
  assert.equal(formatMoney("100000000", true), "¥1.00亿");
});

test("subtractMoney avoids floating-point subtraction drift", () => {
  assert.equal(subtractMoney(5.02, 0.15), "4.87");
  assert.equal(subtractMoney("100.00", "3.00"), "97.00");
});
