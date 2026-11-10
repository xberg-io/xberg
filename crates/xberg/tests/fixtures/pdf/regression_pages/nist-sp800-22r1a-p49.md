**2.11.3 Test Statistic and Reference Distribution**

*2 2 2*

∇ψ

*m(obs)* and∇ ψ *m(obs)* : A measure of how well the observed frequencies of *m* -bit patterns match the expected frequencies of the *m* -bit patterns.

*2*

The reference distribution for the test statistic is the χ distribution.

**2.11.4 Test Description**

- (1) Form an augmented sequence ε′: Extend the sequence by appending the first *m-1* bits to the end of the sequence for distinct values of *n* .

For example, given *n = 10* and ε *= 0011011101* . If *m = 3* , then ε *´* = *001101110100* . If *m = 2* , then ε *´* = *00110111010* . If *m = 1* , then ε *´* = the original sequence *0011011101* .

- (2) Determine the frequency of all possible overlapping *m* -bit blocks, all possible overlapping ( *m-1)* bit blocks and all possible overlapping *(m-2)* -bit blocks. Let *v* denote the frequency of the *m*i ~*1*~ *...i*~*m*~

bit pattern *i*~*1*~*…i ;* ~*m*~ let *v* denote the frequency of the ( *m-1)* -bit pattern *i …i*~*1 m-1*~*;* and let *i*~*1*~ *...i*~*m*~ − *1 v* denote the frequency of the ( *m-2)* -bit pattern *i*~*1*~*…i*~*m-2*~*. i*~*1*~ *...i*~*m*~ − *2* For the example in this section, when *m = 3* , then ( *m-1* ) = 2, and ( *m-2* ) = 1. The frequency of all 3-bit blocks is: *v*~*000*~ *= 0, v*~*001*~ *= 1, v*~*010*~ *= 1, v*~*011*~ *= 2, v*~*100*~ *= 1, v*~*101*~ *= 2*~*, 110*~ *v = 2, v*~*111*~ *= 0.* The frequency of all possible ( *m-1* )-bit blocks is: *v*~*00*~ *= 1, v*~*01*~ *= 3, v*~*10*~ *= 3*~*, 11*~ *v = 3.* The frequency of all ( *m-2* )-bit blocks is: *v*~*0*~ *= 4, v*~*1*~ *= 6.*

*m 2 2* 

- (3) Compute: ψ

= ∑  *vi1*~*...i*~ *m n i*~*1*~ *...i*~*m*~ *2 n*  *2*<sup>*m*</sup> *2* −  = ∑ − *n*

*m*

*v*~*i1 ...i*~ *m* ~*m*~ *2*  *n*

*i*~*1*~ *...i*~*m*~ *m* − *1 2*

*2* ψ~*m*~ = ∑ − *1 n i*~*1*~ *...i*~*m*~ − *1 2*  *n*   *v*~*i*~ −  = ~*1*~ *...i*~*m*~ − *1 m* − *1*  *2*  *m* − *1 2 2* ∑ − *n v*~*i1 ...im*~ − *1 n i*~*1*~ *...i*~*m*~ − *1 m* − *2 2 2* ψ = ∑ ~*m*~ − *2 n i*~*1*~ *...i*~*m*~ − *2*  *n*   *v*~*i*~ − ~*1 ...im*~ − *2 m* − *2*  *2 2 m* − *2 2 2*  = ∑ − *n v*~*i*~ *...i*~*m*~

*1* − *2*

 *n i*~*1*~ *...i*~*m*~ − *2* For the example in this section,

*3 2 2*

ψ

*3 = 10 (0* + *1* + *1* + *4* + *1* + *4* + *4* + *1)* − *10* = *12.8* − *10* = *2.8*

- (4) Compute: ∇ψ =ψ −ψ

2

2

*2*

ψ

*2 =* 10 (1+ 9 + 9 + 9) −10 = 11.2 −10 = 1.2 2 *2*

ψ

*1 =* (16 + 36) −10 = 10.4 −10 = 0.4 10 *2 2 2*

, and

*m m m* − *1 2 2 2 2 2* ∇ = − *2* +

ψ ψ ψ~*m*~ ψ *m m* − *1* .

*m* − *2*

| For the example in ∇ ψ 2 = ψ 2 − ψ 2 = Ψ 2 − Ψ 2 = 2.8 − 1.2 = 1.6 | this | section, |  |  |
| --- | --- | --- | --- | --- |
| m | m | m − 1 | 3 | 2 |
