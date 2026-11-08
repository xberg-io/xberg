is represented by the byte sequence a~0~ a~1~ a~2~ ... a~13~ a~14~ a~15~, (3.4) where a~0~ ={r~0~ r~1~ ... r~7~}; a~1~ ={r~8~ r~9~ ... r~15~}; (3.5) . . a~15~ ={r~120~ r~121~ ... r~127~}.

As described in Section 3.2, the bits within any individual byte are indexed in decreasing order from left to right. This ordering is more natural for the fnite feld arithmetic on bytes that is described in Section 4. The two types of bit indices for byte sequences are illustrated in Table 2.

**Table 2. Indices for bytes and bits**

| Bit index in sequence | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | . . . |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Byte index | 0 |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  | . . . |
| Bit index in byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 | . . . |

# 3.4 The State

Internally, the algorithms for the AES block ciphers are performed on a two-dimensional (fourby-four) array of bytes called the state. In the state array, denoted by s, each individual byte has two indices: a row index r in the range 0 ≤ r \< 4 and a column index c in the range 0 ≤ c \< 4. An individual byte of the state is denoted by either s~r,~c or s[r,c].

In the specifcations for the AES block cipher algorithms in Section 5, the frst step is to copy the input array of bytes in~0~, in~1~, ..., in~15~ to the state array s as follows: s[r,c] = in[r +4c] for 0 ≤ r \< 4 and 0 ≤ c \< 4. (3.6) A sequence of transformations is then applied to the state array, after which its fnal value is copied to the output array of bytes out~0~, out~1~, ..., out~15~ as follows: out[r +4c] = s[r,c] for 0 ≤ r \< 4 and 0 ≤ c \< 4. (3.7) The correspondence between the indices of the input and output with the indices of the state array is illustrated in Fig. 1.
