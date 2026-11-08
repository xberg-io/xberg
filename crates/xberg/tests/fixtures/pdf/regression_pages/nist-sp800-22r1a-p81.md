[1]

[2]

[3]

[4]

*Y*~*i*~

**References for Test**

I. J. Good (1953), "The serial test for sampling numbers and other tests for randomness," Proc.

Cambridge Philos. Soc.. 47, pp. 276-284.

- M. Kimberley (1987), "Comparison of two statistical tests for keystream sequences," Electronics Letters. 23, pp. 365-366.
- D. E. Knuth (1998), The Art of Computer Programming. Vol. 2, 3rd ed. Reading: Addison- Wesley, Inc., pp. 61-80.
- A. J. Menezes, P. C. van Oorschot, and S. A. Vanstone (1997), Handbook of Applied Cryptography. Boca Raton, FL: CRC Press, p. 181.

# ( <sup>*m*</sup>) <sup>=</sup>( <sup>ε</sup>~*i*~,…

**3.12 Approximate Entropy Test**

,ε

## )<sup>, set</sup>

*i* + *m* −1

1

*m*

*C*~*i*~ = *n* +1− *m* Approximate entropy characteristics [1] are based on repeating patterns in the string. If \#{<sup>*j*</sup> :1≤ *j* \< *n* − *m* , *Y*

( <sup>*m*</sup> ) <sup>=</sup> *Y*( <sup>*m*</sup>) }<sup>=</sup> π

*j i l*

| and |  |
| --- | --- |
|  | 1 n + 1 − m |
| Φ ( m ) = | ∑ logCim , |
| n + 1 − m |  |
|  | i + 1 |

logC

*m*

*C*~*i*~ is the relative frequency of occurrences of the pattern *Y*~*i*~( *m* ) in the string, and -Φ

*m*

the empirical distribution arising on the set of all 2 possible patterns of length *m* ,

*m*

2

( *m* )

is the entropy of

( *m* )

Φ = ∑<sup>π</sup> logπ ,

*l l l* =1

where π~*l*~ is the relative frequency of pattern *l* = ( *i*~*1*~, …, *i*~*m*~) in the string.

The approximate entropy *ApEn* of order *m* , *m* ≥ 1 is defined as

( *m* ) ( *m* +1)

*ApEn*( <sup>*m*</sup>) <sup>=</sup> Φ

## − Φ

(1)

with *ApEn* (0) = -Φ . " *ApEn* ( *m* ) measures the logarithmic frequency with which blocks of length *m* that are close together remain close together for blocks augmented by one position. Thus, small values of *ApEn* ( *m* ) imply strong regularity, or persistence, in a sequence. Alternatively, large values of *ApEn* ( *m* ) imply substantial fluctuation, or irregularity." [1, p. 2083].

Pincus and Kalman [2] defined sequence to be *m* -irregular ( *m* -random) if its approximate entropy *ApEn* ( *m* ) takes the largest possible value. They evaluated quantities *ApEn* ( *m* ), *m* = 0, 1, 2 for binary and decimal expansions of *e* , π, 2 and 3 with the surprising conclusion that the expansion of 3 demonstrated more irregularity than that of π.
