const DECIMAL_PATTERN = /^([+-]?)(\d*)(?:\.(\d*))?(?:[eE]([+-]?\d+))?$/;
const MAX_SAFE_EXPONENT = 1_000;

function decimalInputToString(value) {
  if (value === null || value === false || value === "" || value === true) {
    return String(Number(value));
  }

  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed === "" ? "0" : trimmed;
  }
  return String(value);
}

function parseMoneyCents(value) {
  const match = DECIMAL_PATTERN.exec(decimalInputToString(value));
  if (!match || (!match[2] && !match[3])) return null;

  const sign = match[1] === "-" ? -1n : 1n;
  const integerDigits = match[2] || "0";
  const fractionDigits = match[3] || "";
  const exponent = Number.parseInt(match[4] || "0", 10);
  const scale = exponent - fractionDigits.length + 2;

  if (!Number.isSafeInteger(exponent) || Math.abs(scale) > MAX_SAFE_EXPONENT) return null;

  const coefficient = BigInt(`${integerDigits}${fractionDigits}` || "0");
  let absoluteCents;
  let discardedFraction = false;
  if (scale >= 0) {
    absoluteCents = coefficient * 10n ** BigInt(scale);
  } else {
    const divisor = 10n ** BigInt(-scale);
    absoluteCents = coefficient / divisor;
    discardedFraction = coefficient % divisor !== 0n;
  }

  if (sign < 0n && discardedFraction) absoluteCents += 1n;
  return absoluteCents === 0n ? 0n : sign * absoluteCents;
}

function formatScaledHundredths(value) {
  const whole = value / 100n;
  const fraction = (value % 100n).toString().padStart(2, "0");
  return `${whole}.${fraction}`;
}

function divideAndRoundPositive(value, divisor) {
  const quotient = value / divisor;
  const remainder = value % divisor;
  return remainder * 2n >= divisor ? quotient + 1n : quotient;
}

function formatFullCents(cents) {
  const negative = cents < 0n;
  const absolute = negative ? -cents : cents;
  const whole = (absolute / 100n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const fraction = (absolute % 100n).toString().padStart(2, "0");
  return `¥${negative ? "-" : ""}${whole}.${fraction}`;
}

function formatInvalidMoney(value, abbreviate) {
  const numeric = Number(value);
  if (numeric >= 100000000 && abbreviate) return `¥${numeric / 100000000}亿`;
  if (numeric >= 10000 && abbreviate) return `¥${numeric / 10000}万`;
  if (Number.isFinite(numeric)) {
    return `¥${numeric.toFixed(2).replace(/\B(?=(\d{3})+(?!\d))/g, ",")}`;
  }
  return `¥${numeric}`;
}

export const formatMoney = (value, abbreviate = false) => {
  const cents = parseMoneyCents(value);
  if (cents === null) return formatInvalidMoney(value, abbreviate);

  if (abbreviate && cents >= 10_000_000_000n) {
    const hundredthsOfYi = divideAndRoundPositive(cents, 100_000_000n);
    return `¥${formatScaledHundredths(hundredthsOfYi)}亿`;
  }
  if (abbreviate && cents >= 1_000_000n) {
    const hundredthsOfWan = divideAndRoundPositive(cents, 10_000n);
    return `¥${formatScaledHundredths(hundredthsOfWan)}万`;
  }

  return formatFullCents(cents);
};

export const subtractMoney = (left, right) => {
  const leftCents = parseMoneyCents(left);
  const rightCents = parseMoneyCents(right);
  if (leftCents === null || rightCents === null) return "0.00";

  const result = leftCents - rightCents;
  const negative = result < 0n;
  const absolute = negative ? -result : result;
  return `${negative ? "-" : ""}${absolute / 100n}.${(absolute % 100n)
    .toString()
    .padStart(2, "0")}`;
};
