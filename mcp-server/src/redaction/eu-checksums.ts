/**
 * Validate a Polish PESEL national ID checksum.
 * Mod-10 algorithm with official weights from Polish GUS.
 * https://en.wikipedia.org/wiki/PESEL
 */
export function isValidPesel(pesel: string): boolean {
  if (!/^\d{11}$/.test(pesel)) return false;
  const digits = pesel.split("").map(Number);
  const weights = [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
  const sum = weights.reduce((acc, w, i) => acc + w * (digits[i] ?? 0), 0);
  const checkDigit = digits[10] ?? 0;
  const expected = (10 - (sum % 10)) % 10;
  return expected === checkDigit;
}

/**
 * Validate a Dutch BSN national ID checksum.
 * Mod-11 algorithm per official Dutch RvIG specification; last digit has weight -1.
 * https://en.wikipedia.org/wiki/Burgerservicenummer
 */
export function isValidBsn(bsn: string): boolean {
  if (!/^\d{9}$/.test(bsn)) return false;
  const digits = bsn.split("").map(Number);
  const weights = [9, 8, 7, 6, 5, 4, 3, 2, -1];
  const sum = weights.reduce((acc, w, i) => acc + w * (digits[i] ?? 0), 0);
  return sum % 11 === 0;
}

/**
 * Validate a Belgian Registre National checksum (97-modulo).
 * Format: YYMMDDXXXXX (6-digit birth date + 3-digit sequence + 2-digit check).
 * Century is ambiguous from YY alone, so both the pre-2000 and post-2000
 * formulas are tried.
 */
export function isValidBelgianRegistre(num: string): boolean {
  if (!/^\d{11}$/.test(num)) return false;
  const n = Number(num.slice(0, 9));
  const checkActual = Number(num.slice(9, 11));
  if (97 - (n % 97) === checkActual) return true;
  return 97 - ((2_000_000_000 + n) % 97) === checkActual;
}

/**
 * Validate a French INSEE / NIR (numéro de sécurité sociale) checksum.
 * Format: sex(1) + year(2) + month(2) + department(2) + commune(3) + order(3) + key(2).
 * Key = 97 - (first 13 digits mod 97). Corsica's letter department codes
 * (2A/2B) are out of scope -- the source regex only matches numeric departments.
 * https://en.wikipedia.org/wiki/INSEE_code
 */
export function isValidFrInsee(nir: string): boolean {
  const digits = nir.replace(/\s/g, "");
  if (!/^\d{15}$/.test(digits)) return false;
  const base = Number(digits.slice(0, 13));
  const key = Number(digits.slice(13, 15));
  return 97 - (base % 97) === key;
}

const ES_DNI_LETTERS = "TRWAGMYFPDXBNJZSQVHLCKE";

/**
 * Validate a Spanish DNI/NIE checksum (mod-23 letter lookup).
 * DNI: 8 digits + letter. NIE: leading X/Y/Z (mapped to 0/1/2) + 7 digits + letter.
 * https://en.wikipedia.org/wiki/Documento_Nacional_de_Identidad_(Spain)
 */
export function isValidEsDni(id: string): boolean {
  const cleaned = id.toUpperCase().replace(/\s/g, "");
  const match = /^([0-9XYZ])(\d{7})([A-Z])$/.exec(cleaned);
  if (!match) return false;
  const prefix = match[1] ?? "";
  const rest = match[2] ?? "";
  const letter = match[3] ?? "";
  const nieDigit: Record<string, string> = { X: "0", Y: "1", Z: "2" };
  const fullNumber = Number((nieDigit[prefix] ?? prefix) + rest);
  return ES_DNI_LETTERS[fullNumber % 23] === letter;
}

const CF_ODD_VALUES: Record<string, number> = {
  "0": 1, "1": 0, "2": 5, "3": 7, "4": 9, "5": 13, "6": 15, "7": 17, "8": 19, "9": 21,
  A: 1, B: 0, C: 5, D: 7, E: 9, F: 13, G: 15, H: 17, I: 19, J: 21,
  K: 2, L: 4, M: 18, N: 20, O: 11, P: 3, Q: 6, R: 8, S: 12, T: 14,
  U: 16, V: 10, W: 22, X: 25, Y: 24, Z: 23,
};

const CF_EVEN_VALUES: Record<string, number> = {
  "0": 0, "1": 1, "2": 2, "3": 3, "4": 4, "5": 5, "6": 6, "7": 7, "8": 8, "9": 9,
  A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6, H: 7, I: 8, J: 9,
  K: 10, L: 11, M: 12, N: 13, O: 14, P: 15, Q: 16, R: 17, S: 18, T: 19,
  U: 20, V: 21, W: 22, X: 23, Y: 24, Z: 25,
};

/**
 * Validate an Italian Codice Fiscale checksum.
 * Sums per-position values (odd/even 1-indexed lookup tables) over the first
 * 15 characters; the mod-26 remainder maps to the 16th (control) letter.
 * https://en.wikipedia.org/wiki/Italian_fiscal_code_card
 */
export function isValidItCodiceFiscale(cf: string): boolean {
  const code = cf.toUpperCase();
  if (!/^[A-Z]{6}\d{2}[A-Z]\d{2}[A-Z]\d{3}[A-Z]$/.test(code)) return false;
  let sum = 0;
  for (let i = 0; i < 15; i++) {
    const ch = code[i] ?? "";
    sum += i % 2 === 0 ? (CF_ODD_VALUES[ch] ?? 0) : (CF_EVEN_VALUES[ch] ?? 0);
  }
  return String.fromCharCode(65 + (sum % 26)) === code[15];
}

/**
 * Standard Luhn checksum, used to validate French SIRET/SIREN numbers.
 * https://en.wikipedia.org/wiki/Luhn_algorithm
 */
export function isValidLuhn(digits: string): boolean {
  if (digits.length === 0) return false;
  let sum = 0;
  for (let i = 0; i < digits.length; i++) {
    const ch = digits[digits.length - 1 - i];
    if (ch === undefined || !/\d/.test(ch)) return false;
    let d = Number(ch);
    if (i % 2 === 1) {
      d *= 2;
      if (d > 9) d -= 9;
    }
    sum += d;
  }
  return sum % 10 === 0;
}
