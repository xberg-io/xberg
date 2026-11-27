# 4\. Mathematical Preliminaries

For some transformations of the AES algorithms specifed in Sec. 5, each byte in the state array is interpreted as one of the 256 elements of a fnite feld, also known as a Galois Field, denoted

8 1

by GF(2 ).

In order to defne addition and multiplication in GF(2<sup>8</sup>), each byte {b~7~ b~6~ b~5~ b~4~ b~3~ b~2~ b~1~ b~0~} is interpreted as a polynomial, denoted by b(x), as follows:

7 6 5 4 3

b(x) = b~7~ x +b~6~ x +b~5~ x +b~4~ x +b~3~ x +b~2~ x<sup>2</sup> +b~1~ x+b~0~. (4.1)

6 5

For example, {01100011} is represented by the polynomial x +x +x+1.

## **4.1 Addition in GF(2**<sup>**8**</sup>**)**

In order to add two elements in the fnite feld GF(2<sup>8</sup>), the coeffcients of the polynomials that represent the elements are added modulo 2 (i.e., with the exclusive-OR operation, denoted by ⊕), so that 1⊕1 = 0, 1⊕0 = 1, and 0 ⊕0 = 0.

Equivalently, two bytes can be added by applying the exclusive-OR operation to each pair of corresponding bits in the bytes. Thus, the sum of {a a a~5~ a a a~2~ a a~0~} and {b~7~ b~6~ b~5~ b~4~ b~3~ b~2~ b~1~ b~0~} is $\\{a_{7}\oplus b_{7}a_{6}\oplus b_{6}a_{5}\oplus b_{5}a_{4}\oplus b_{4}a_{3}\oplus b_{3}^{7}a_{2}^{6}\oplus b_{2}^{4}a_{1}^{3}\oplus b_{1}^{1}a_{0}\oplus b_{0}\\}$. (In Section 5.1.4, this defnition is extended to words.) For example, the following three representations of addition are equivalent:

6 4 2 7 7 6 4

$(x +x +x +x+1) + (x +x+1) = x +x +x +x^{2}$ (polynomial)

$$
\{01010111\}\oplus\{10000011\}=\{11010100\}
$$

(binary) (4.2)

$$
\{57\}\oplus\{83\}=\{d4\}
$$

(hexadecimal).

Because the coeffcients of the polynomials are reduced modulo 2, the coeffcient 1 is equivalent

4 2

to the coeffcient –1, so addition is equivalent to subtraction. For example, x +x represents the same fnite feld element as $x^{4}-x^{2},-x^{4}+x^{2}$, and $-x^{4}-x^{2}$. Similarly, the sum of any element with itself is the zero element.

## **4.2 Multiplication in GF(2**<sup>**8**</sup>**)**

8

The symbol · denotes multiplication in GF 2 . Conceptually, this multiplication is defned on two bytes in two steps: 1) the two polynomials that represent the bytes are multiplied as polynomials, and 2) the resulting polynomial is reduced modulo the following fxed polynomial:

8 4

$m(x) = x +x +x^{3} +x+1. (4.3)$ Within both steps, the individual coeffcients of the polynomials are reduced modulo 2.

1

Information about the properties of fnite felds can be found in textbooks, such as Michael Artin's Algebra [4].
