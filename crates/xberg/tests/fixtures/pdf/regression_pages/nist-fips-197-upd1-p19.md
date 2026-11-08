# 5\. Algorithm Specifcations

The general function for executing AES-128, AES-192, or AES-256 is denoted by CIPHER(); its

2

inverse is denoted by INVCIPHER().

The core of the algorithms for CIPHER() and INVCIPHER() is a sequence of fxed transformations of the state called a round. Each round requires an additional input called the round key; the round key is a block that is usually represented as a sequence of four words (i.e., 16 bytes).

An expansion routine, denoted by KEYEXPANSION(), takes the block cipher key as input and generates the round keys as output. In particular, the input to KEYEXPANSION() is represented as an array of words, denoted by key, and the output is an expanded array of words, denoted by w, called the key schedule.

The block ciphers AES-128, AES-192, and AES-256 differ in three respects: 1) the length of the key; 2) the number of rounds, which determines the size of the required key schedule; and 3) the specifcation of the recursion within KEYEXPANSION(). For each algorithm, the number of rounds is denoted by Nr, and the number of words of the key is denoted by Nk. (The number of words in the state is denoted by Nb for Rijndael in general; in this Standard, Nb = 4.) The specifc values of Nk, Nb, and Nr are given in Table 3. No other confgurations of Rijndael conform to this Standard.

For implementation issues relating to the key length, block size, and number of rounds, see

Section 6.3.

**Table 3. Key-Block-Round Combinations**

|  | Key length Nk | (in bits) | Block size Nb | (in bits) Number | of rounds Nr |
| --- | --- | --- | --- | --- | --- |
| AES-128 | 4 | 128 | 4 | 128 | 10 |
| AES-192 | 6 | 192 | 4 | 128 | 12 |
| AES-256 | 8 | 256 | 4 | 128 | 14 |

The three inputs to CIPHER() are: 1) the data input in, which is a block represented as a linear array of 16 bytes; 2) the number of rounds Nr for the instance; and 3) the round keys. Thus, AES-128(in,key) = CIPHER(in,10,KEYEXPANSION(key)) AES-192(in,key) = CIPHER(in,12,KEYEXPANSION(key)) (5.1) AES-256(in,key) = CIPHER(in,14,KEYEXPANSION(key)).

The inverse permutations are defned by replacing CIPHER() with INVCIPHER() in Eq. 5.1.

2

Informally, these functions are sometimes called "encryption" and "decryption," but neutral terminology is appropriate because there are other applications of block ciphers besides encryption.
