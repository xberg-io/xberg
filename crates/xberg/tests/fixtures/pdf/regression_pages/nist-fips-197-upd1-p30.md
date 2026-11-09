**Algorithm 3** Pseudocode for INVCIPHER()

1: **procedure** INVCIPHER(in, Nr, w) 2: state ← in . See Sec. 3.4 3: state ← ADDROUNDKEY(state,w[4 ∗Nr..4 ∗Nr +3]) . See Sec. 5.1.4 4: **for** round **from** Nr −1 **downto** 1 **do** 5: state ← INVSHIFTROWS(state) . See Sec. 5.3.1 6: state ← INVSUBBYTES(state) . See Sec. 5.3.2 7: state ← ADDROUNDKEY(state,w[4 ∗round..4 ∗round +3]) 8: state ← INVMIXCOLUMNS(state) . See Sec. 5.3.3 9:

**end for**

10: state ← INVSHIFTROWS(state) 11: state ← INVSUBBYTES(state) 12: state ← ADDROUNDKEY(state,w[0..3]) 13: **return** state 14: **end procedure**

**5.3.1 INVSHIFTROWS()**

INVSHIFTROWS() is the inverse of the SHIFTROWS(). In particular, the bytes in the last three rows of the state are cyclically shifted as follows:

0

s~r,~c = s~r,~(c−r) mod 4 for 0 ≤ r \< 4 and 0 ≤ c \< 4. (5.12) INVSHIFTROWS() is illustrated in Figure 9. In that representation of the state, the effect is to move each byte by r positions to the right in the row, cycling the right-most r bytes around to the left end of the row. The frst row, where r = 0, is unchanged.
