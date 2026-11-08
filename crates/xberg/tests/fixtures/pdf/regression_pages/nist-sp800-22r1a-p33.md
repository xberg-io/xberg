For example, if *n = 20* , *M = Q = 3* , and ε *= 01011001001010101101* , then partition the stream

 *n* 

into N =   = 2 matrices of cardinality M

- Q (3
- 3 = 9). Note that the last two bits (0 and 1)

 *3* · *3* 

*0 1 0 0 1 0* will be discarded. The two matrices are *1 1 0* and *1 0 1* . Note that the first matrix *0 1 0 0 1 1* consists of the first three bits in row 1, the second set of three bits in row 2, and the third set of three bits in row 3. The second matrix is similarly constructed using the next nine bits in the sequence.

- (2) Determine the binary rank ( *R*~~) of each matrix, where  = 1,..., *N* . The method for determining the rank is described in Appendix A.

For the example in this section, the rank of the first matrix is 2 ( *R*~*1*~ *= 2* ), and the rank of the second matrix is 3 ( *R*~*2*~ *= 3* ).

- (3) Let *F*~*M*~ = the number of matrices with *R*~~= *M* (full rank),

*F*~*M-1*~ = the number of matrices with *R*~~= *M-1* (full rank - 1), *N – F*~*M*~ *- F*~*M-1*~ = the number of matrices remaining.

For the example in this section, *F*~*M*~ *= F*~*3*~ *= 1* ( *R*~*2*~ has the full rank of 3 *), F*~*M-1*~ *= F*~*2*~ *= 1* ( *R*~*1*~ has rank 2), and no matrix has any lower rank.

- (4) Compute

2 2

( *F* − 0.2888 *N* ) ( *F* − 0.5776 *N* ) ( *N* − *F* − *F* − 0.1336 *N* )<sup>2</sup>

2 *M M* −1 *M M* −1

χ ( *obs* ) = + + .

0.2888 *N* 0.5776 *N* 0.1336 *N*

2

χ ( *obs* ) = For the example in this section,

2

# (~1~− 0.2888· 2)

\+

0.2888· 2

2 −χ ( *obs* ) / 2

- (5) Compute *P* − *value* = *e*

2

example is equal to *igamc* 1,  

2

# (~1~− 0.5776 · 2)

\+

0.5776 · 2

 χ ( *obs* )   . 2 

2

# (~2~ −1−1− 0.1336 · 2)

*= 0.596953.*

0.1336 · 2

. Since there are 3 classes in the example, the *P-value* for the

For the example in this section, *P-value = e*

**2.5.5 Decision Rule (at the 1% Level)**

that the sequence is random.

*0.596953 2*

= 0.741948.

If the computed *P-value* is \< 0.01, then conclude that the sequence is non-random. Otherwise, conclude
