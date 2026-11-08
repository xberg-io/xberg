| 24 | 11f915bc | f915bc11 | 99596582 | 20000000 | b9596582 | d4d1c6f8 | 6d88a37a |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 25 | 6d88a37a |  |  |  |  | 7c839d87 | 110b3efd |
| 26 | 110b3efd |  |  |  |  | caf2b8bc | dbf98641 |
| 27 | dbf98641 |  |  |  |  | 11f915bc | ca0093fd |
| 28 | ca0093fd | 0093fdca | 63dc5474 | 40000000 | 23dc5474 | 6d88a37a | 4e54f70e |
| 29 | 4e54f70e |  |  |  |  | 110b3efd | 5f5fc9f3 |
| 30 | 5f5fc9f3 |  |  |  |  | dbf98641 | 84a64fb2 |
| 31 | 84a64fb2 |  |  |  |  | ca0093fd | 4ea6dc4f |
| 32 | 4ea6dc4f | a6dc4f4e | 2486842f | 80000000 | a486842f | 4e54f70e | ead27321 |
| 33 | ead27321 |  |  |  |  | 5f5fc9f3 | b58dbad2 |
| 34 | b58dbad2 |  |  |  |  | 84a64fb2 | 312bf560 |
| 35 | 312bf560 |  |  |  |  | 4ea6dc4f | 7f8d292f |
| 36 | 7f8d292f | 8d292f7f | 5da515d2 | 1b000000 | 46a515d2 | ead27321 | ac7766f3 |
| 37 | ac7766f3 |  |  |  |  | b58dbad2 | 19fadc21 |
| 38 | 19fadc21 |  |  |  |  | 312bf560 | 28d12941 |
| 39 | 28d12941 |  |  |  |  | 7f8d292f | 575c006e |
| 40 | 575c006e | 5c006e57 | 4a639f5b | 36000000 | 7c639f5b | ac7766f3 | d014f9a8 |

| 41 | d014f9a8 | 19fadc21 c9ee2589 |
| --- | --- | --- |
| 42 | c9ee2589 | 28d12941 e13f0cc8 |
| 43 | e13f0cc8 | 575c006e b6630ca6 |

# A.2 Expansion of a 192-bit Key

## This section contains the key expansion of the following key:

Key = 8e 73 b0 f7 da 0e 64 52 c8 10 f3 2b 80 90 79 e5 62 f8 ea d2 52 2c 6b 7b for Nk = 6, which results in

w~0~ = 8e73b0f7 w~1~ = da0e6452 w~2~ = c810f32b

w~3~ = 809079e5 w~4~ = 62f8ead2 w~5~ = 522c6b7b

w[i] = i After After After XOR temp Rcon[i/Nk] w[i−Nk] temp⊕

- (dec) RO TWO R D() SU BWO R D() with Rcon

w[i−Nk]

| 6 | 522c6b7b 2c6b7b52 717f2100 01000000 707f2100 | 8e73b0f7 | fe0c91f7 |
| --- | --- | --- | --- |
| 7 | fe0c91f7 | da0e6452 | 2402f5a5 |
| 8 | 2402f5a5 | c810f32b | ec12068e |
