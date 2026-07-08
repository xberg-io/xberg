import { createCipheriv, createDecipheriv, scryptSync, randomBytes } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

const MAGIC = Buffer.from("XPII\x01");
const SALT_LEN = 16;
const IV_LEN = 12;
const TAG_LEN = 16;
const KEY_LEN = 32;

function deriveKey(passphrase: string, salt: Buffer): Buffer {
  return scryptSync(passphrase, salt, KEY_LEN) as Buffer;
}

export function encryptMapFile(
  filePath: string,
  tokenMap: Record<string, string>,
  passphrase: string
): void {
  const plain = JSON.stringify(tokenMap);
  const salt = randomBytes(SALT_LEN);
  const iv = randomBytes(IV_LEN);
  const key = deriveKey(passphrase, salt);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  const enc = Buffer.concat([cipher.update(plain, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  writeFileSync(filePath, Buffer.concat([MAGIC, salt, iv, tag, enc]));
}

/**
 * @deprecated The `rehydrate_document` tool now decrypts rehydration maps
 * in-wasm via `engine.decrypt_map` for parity with the Rust/wasm crypto
 * implementation. This TS Node-crypto path is retained for direct callers
 * and test coverage of the on-disk wire format, but is no longer used by
 * any MCP tool handler.
 */
export function decryptMapFile(
  filePath: string,
  passphrase: string
): Record<string, string> {
  const raw = readFileSync(filePath);
  if (raw.subarray(0, MAGIC.length).equals(MAGIC)) {
    const off = MAGIC.length;
    const salt = raw.subarray(off, off + SALT_LEN);
    const iv = raw.subarray(off + SALT_LEN, off + SALT_LEN + IV_LEN);
    const tag = raw.subarray(off + SALT_LEN + IV_LEN, off + SALT_LEN + IV_LEN + TAG_LEN);
    const data = raw.subarray(off + SALT_LEN + IV_LEN + TAG_LEN);
    const key = deriveKey(passphrase, salt);
    const decipher = createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAuthTag(tag);
    const dec = Buffer.concat([decipher.update(data), decipher.final()]);
    return JSON.parse(dec.toString("utf-8")) as Record<string, string>;
  }
  // Plaintext fallback for maps saved without a passphrase
  return JSON.parse(raw.toString("utf-8")) as Record<string, string>;
}
