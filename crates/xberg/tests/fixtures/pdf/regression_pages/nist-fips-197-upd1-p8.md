# List of Tables List of Figures

| Table 1 | Hexadecimal representation of 4-bit sequences . . . . . . . . . . . . . . | 5 |
| --- | --- | --- |
| Table 2 | Indices for bytes and bits . . . . . . . . . . . . . . . . . . . . . . . . . . | 6 |
| Table 3 | Key-Block-Round Combinations . . . . . . . . . . . . . . . . . . . . . . . 11 |  |
| Table 4 | SBOX(): substitution values for the byte xy (in hexadecimal format) . . . | 14 |
| Table 5 | Round constants . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . | 17 |
| Table 6 | INVSBOX(): substitution values for the byte xy (in hexadecimal format) | 23 |

| Figure 1 |  | State array input and output . . . . . . . . . . . . . . . . . . . . . . . . | 7 |
| --- | --- | --- | --- |
| Figure 2 |  | Illustration of SUBBYTES() . . . . . . . . . . . . . . . . . . . . . . . . | 13 |
| Figure 3 |  | Illustration of SHIFTROWS() . . . . . . . . . . . . . . . . . . . . . . . | 15 |
| Figure 4 |  | Illustration of MIXCOLUMNS() . . . . . . . . . . . . . . . . . . . . . . | 16 |
| Figure 5 |  | Illustration of ADDROUNDKEY() . . . . . . . . . . . . . . . . . . . . . | 16 |
| Figure 6 |  | KEYEXPANSION() of AES-128 to generate the words w [i] for 4 ≤ i \< 44, |  |
|  | where l ranges over the multiples of 4 between 0 and 36 . . . . . . . . . . . . . |  | 19 |
| Figure 7 |  | KEYEXPANSION() of AES-192 to generate the words w [i] for 6 ≤ i \< 52, |  |
|  | where l ranges over the multiples of 6 between 0 and 42 . . . . . . . . . . . . . |  | 20 |
| Figure 8 |  | KEYEXPANSION() of AES-256 to generate the words w [i] for 8 ≤ i \< 60, |  |
|  | where l ranges over the multiples of 8 between 0 and 48 . . . . . . . . . . . . . . 21 |  |  |
| Figure 9 |  | Illustration of INVSHIFTROWS() . . . . . . . . . . . . . . . . . . . . . | 23 |

| List of Algorithms |  |  |
| --- | --- | --- |
| Algorithm 1 | Pseudocode for CIPHER() . . . . . . . . . . . . . . . . . . . . . . . | 12 |
| Algorithm 2 | Pseudocode for KEYEXPANSION() . . . . . . . . . . . . . . . . . . | 18 |
| Algorithm 3 | Pseudocode for INVCIPHER() . . . . . . . . . . . . . . . . . . . . . | 22 |
| Algorithm 4 | Pseudocode for EQINVCIPHER() . . . . . . . . . . . . . . . . . . . | 25 |
| Algorithm 5 | Pseudocode for KEYEXPANSIONEIC() . . . . . . . . . . . . . . . | 25 |
