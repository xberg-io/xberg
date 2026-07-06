import { describe, it, expect } from "vitest";
import {
  isValidPesel,
  isValidBsn,
  isValidBelgianRegistre,
  isValidFrInsee,
  isValidEsDni,
  isValidItCodiceFiscale,
  isValidLuhn,
} from "../src/redaction/eu-checksums.js";

describe("isValidPesel", () => {
  it("accepts a valid PESEL", () => {
    // sum(d*w for weights [1,3,7,9,1,3,7,9,1,3]) = 89, check = (10-9)%10 = 1
    expect(isValidPesel("80051501231")).toBe(true);
  });

  it("rejects a PESEL with a wrong check digit", () => {
    expect(isValidPesel("80051501230")).toBe(false);
  });

  it("rejects the wrong length", () => {
    expect(isValidPesel("8005150123")).toBe(false);
    expect(isValidPesel("800515012345")).toBe(false);
  });

  it("rejects non-digit input", () => {
    expect(isValidPesel("8005150123X")).toBe(false);
  });
});

describe("isValidBsn", () => {
  it("accepts a valid BSN", () => {
    expect(isValidBsn("123456782")).toBe(true);
  });

  it("rejects a BSN with a wrong check digit", () => {
    expect(isValidBsn("123456780")).toBe(false);
  });

  it("rejects the wrong length", () => {
    expect(isValidBsn("12345678")).toBe(false);
  });
});

describe("isValidBelgianRegistre", () => {
  it("accepts a valid pre-2000 Registre National number", () => {
    // 800515012 % 97 = 8, check = 97 - 8 = 89
    expect(isValidBelgianRegistre("80051501289")).toBe(true);
  });

  it("rejects a pre-2000 number with the wrong check digits", () => {
    expect(isValidBelgianRegistre("80051501294")).toBe(false);
  });

  it("accepts a valid post-2000 Registre National number", () => {
    // Born 2001-05-15, sequence 012: n = 010515012
    // 2_000_000_000 % 97 = 68; 10_515_012 % 97 = 18; (68+18)%97 = 86; check = 97-86 = 11
    expect(isValidBelgianRegistre("01051501211")).toBe(true);
  });

  it("rejects the wrong length", () => {
    expect(isValidBelgianRegistre("8005150128")).toBe(false);
  });
});

describe("isValidFrInsee", () => {
  it("accepts a valid NIR (key = 97 - (base mod 97))", () => {
    // base 1850575116023 % 97 = 60, key = 97 - 60 = 37
    expect(isValidFrInsee("185057511602337")).toBe(true);
  });

  it("rejects a NIR-shaped number with a wrong check key", () => {
    expect(isValidFrInsee("185057511602324")).toBe(false);
  });

  it("rejects the wrong length", () => {
    expect(isValidFrInsee("18505751160233")).toBe(false);
    expect(isValidFrInsee("1850575116023377")).toBe(false);
  });

  it("rejects non-digit input", () => {
    expect(isValidFrInsee("18505751160233X")).toBe(false);
  });
});

describe("isValidEsDni", () => {
  it("accepts a valid DNI", () => {
    expect(isValidEsDni("12345678Z")).toBe(true);
  });

  it("rejects a DNI with the wrong check letter", () => {
    expect(isValidEsDni("12345678A")).toBe(false);
  });

  it("accepts a valid NIE (X/Y/Z prefix mapped to 0/1/2)", () => {
    expect(isValidEsDni("X1234567L")).toBe(true);
  });

  it("rejects malformed input", () => {
    expect(isValidEsDni("1234567Z")).toBe(false);
    expect(isValidEsDni("123456789")).toBe(false);
  });
});

describe("isValidItCodiceFiscale", () => {
  it("accepts a valid Codice Fiscale", () => {
    expect(isValidItCodiceFiscale("RSSMRA85T10A562S")).toBe(true);
  });

  it("rejects a Codice Fiscale with the wrong control letter", () => {
    expect(isValidItCodiceFiscale("RSSMRA85T10A562A")).toBe(false);
  });

  it("rejects malformed input", () => {
    expect(isValidItCodiceFiscale("RSSMRA85T10A562")).toBe(false);
    expect(isValidItCodiceFiscale("1234567890123456")).toBe(false);
  });
});

describe("isValidLuhn", () => {
  it("accepts a valid SIRET (Luhn over all 14 digits)", () => {
    expect(isValidLuhn("73282932000074")).toBe(true);
  });

  it("accepts a valid SIREN (Luhn over the 9-digit prefix)", () => {
    expect(isValidLuhn("732829320")).toBe(true);
  });

  it("rejects a digit string with a bad checksum", () => {
    expect(isValidLuhn("73282932000075")).toBe(false);
  });

  it("rejects empty or non-digit input", () => {
    expect(isValidLuhn("")).toBe(false);
    expect(isValidLuhn("7328293200007X")).toBe(false);
  });
});
