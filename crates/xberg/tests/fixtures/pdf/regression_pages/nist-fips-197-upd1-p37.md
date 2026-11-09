# Appendix A — Key Expansion Examples

This appendix shows the development of the key schedule for each key size. Note that multi-byte values are presented using the notation described in Sec. 3. The intermediate values produced during the development of the key schedule (see Sec. 5.2) are given in the following table (all values are in hexadecimal format with the exception of the index column (i)).

# A.1 Expansion of a 128-bit Key

## This section contains the key expansion of the following key:

Key = 2b 7e 15 16 28 ae d2 a6 ab f7 15 88 09 cf 4f 3c for Nk = 4, which results in

w~0~ = 2b7e1516 w~1~ = 28aed2a6 w~2~ = abf71588 w~3~ = 09cf4f3c

w[i] = i After After After XOR temp Rcon[i/Nk] w[i−Nk] temp⊕

- (dec) ROTWORD() SUBWORD() with Rcon w[i−Nk]

| 4 | 09cf4f3c | cf4f3c09 | 8a84eb01 | 01000000 | 8b84eb01 | 2b7e1516 | a0fafe17 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 5 | a0fafe17 |  |  |  |  | 28aed2a6 | 88542cb1 |
| 6 | 88542cb1 |  |  |  |  | abf71588 | 23a33939 |
| 7 | 23a33939 |  |  |  |  | 09cf4f3c | 2a6c7605 |
| 8 | 2a6c7605 | 6c76052a | 50386be5 | 02000000 | 52386be5 | a0fafe17 | f2c295f2 |
| 9 | f2c295f2 |  |  |  |  | 88542cb1 | 7a96b943 |
| 10 | 7a96b943 |  |  |  |  | 23a33939 | 5935807a |
| 11 | 5935807a |  |  |  |  | 2a6c7605 | 7359f67f |
| 12 | 7359f67f | 59f67f73 | cb42d28f | 04000000 | cf42d28f | f2c295f2 | 3d80477d |
| 13 | 3d80477d |  |  |  |  | 7a96b943 | 4716fe3e |
| 14 | 4716fe3e |  |  |  |  | 5935807a | 1e237e44 |
| 15 | 1e237e44 |  |  |  |  | 7359f67f | 6d7a883b |
| 16 | 6d7a883b | 7a883b6d | dac4e23c | 08000000 | d2c4e23c | 3d80477d | ef44a541 |
| 17 | ef44a541 |  |  |  |  | 4716fe3e | a8525b7f |
| 18 | a8525b7f |  |  |  |  | 1e237e44 | b671253b |
| 19 | b671253b |  |  |  |  | 6d7a883b | db0bad00 |
| 20 | db0bad00 | 0bad00db | 2b9563b9 | 10000000 | 3b9563b9 | ef44a541 | d4d1c6f8 |

| 21 | d4d1c6f8 | a8525b7f 7c839d87 |
| --- | --- | --- |
| 22 | 7c839d87 | b671253b caf2b8bc |
| 23 | caf2b8bc | db0bad00 11f915bc |
