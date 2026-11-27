| Algorithm | 2 Pseudocode for KEYEXPANSION() |  |
| --- | --- | --- |
| 1: procedure | KEYEXPANSION( key) |  |
| 2: i ← 0 |  |  |
| 3: while i | ≤ Nk − 1 do |  |
| 4: | w [i] ← key [4 ∗ i .. 4 ∗ i + 3] |  |
| 5: | i ← i + 1 |  |
| 6: end while |  | . When the loop concludes, i = Nk . |
| 7: while i | ≤ 4 ∗ Nr + 3 do |  |
| 8: | temp ← w [i − 1] |  |
| 9: | if i mod Nk = 0 then |  |
| 10: | temp ← SUBWORD( ROTWORD( temp)) ⊕ Rcon [i / Nk] |  |
| 11: | else if Nk \> 6 and i mod Nk = 4 then |  |
| 12: | temp ← SUBWORD( temp) |  |
| 13: | end if |  |
| 14: | w [i] ← w [i − Nk] ⊕ temp |  |
| 15: | i ← i + 1 |  |
| 16: end while |  |  |
| 17: return | w |  |
| 18: end procedure |  |  |

$$
i\leftarrow i+1
$$

$$
i\leftarrow i+1
$$

Figures 6, 7, and 8 illustrate KEYEXPANSION() for AES-128, AES-192, and AES-256.

# 5.3 INVCIPHER()

To implement INVCIPHER(), the transformations in the specifcation of CIPHER() (Section 5.1) are inverted and executed in reverse order. The inverted transformations of the state — denoted by INVSHIFTROWS(), INVSUBBYTES(), INVMIXCOLUMNS(), and ADDROUNDKEY() — are described in Sections 5.3.1–5.3.4.

INVCIPHER() is described in the pseudocode in Alg. 3, where the array w denotes the key schedule, as described in Section 5.2.
