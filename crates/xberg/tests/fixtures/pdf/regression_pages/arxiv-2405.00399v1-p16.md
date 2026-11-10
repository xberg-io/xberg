And considering (3.22), we have the following estimate s

(ℓ+1)

η<sup>2</sup>(W~H~) 1 ¯

a

u¯~i,h~ − E u¯~i,h~ ≤ λ~i,h~ 1 + 1 + i,h

(ℓ+1) 2 (ℓ) a,h

λ~1~ δ λδ ~λ 1 λ~ The following ∥·∥ -error estimate holds ~b~ \! 1 u¯~i,h~

(ℓ+1)

− E~i,h~ u¯~i,h~ ≤

b

From (3.23) and (3.24), the proof is completed.

the following error estimates u¯~i,h~ − E

(ℓ+1)

u¯~i,h~ ≤

i,h a,h (ℓ+1)

u¯~i,h~ − E~i,h~ u¯~i,h~ ≤

b

1 +

(ℓ+1) a

λδ~1 λ~

**Corollary 3.1.** Under the conditions of Theorem 3.2, the eigenfunction approximation u

ℓ

¯ γ(λ~i,h~) u¯~i,h~ − E \! 1 1 +

a (ℓ+1)

λδ~1 λ~ where s η<sup>2</sup>(W~H~) 1 ¯ ¯

a

γ(λ~i,h~) = λ~i,h~ 1 + 1 + 2

(ℓ+1)

λ~1~ δ λδ ~λ 1~

(ℓ)

The error estimate for the eigenvalue approximation λ~i,h~

Remark 3.2.

# 4 Numerical experiments

\!

2

η~a~(W~H~) u¯

η (W~H~) u¯~i,h~ − E

(ℓ) i,h

− E~i,h~u¯~i,h~ .(3.23)

a,h

(ℓ+1)

u¯~i,h~ . (3.24)

i,h a,h

- (1) u¯~i,h~ ,

i,h a,h (ℓ+1)

η (W~H~) u¯~i,h~ − E~i,h~

\!

2

η~a~(W~H~).

(ℓ) λ

(ℓ+1)

has

i,h

(3.25) u¯~i,h~ , (3.26)

a,h

(3.27)

can be deduced from Theorem 3.2 and

In this section, numerical experiments are presented to validate our theoretical results. Here, we are concerned with the Laplace eigenvalue problem (2.1), where the computing domain is set to be the unit square Ω = (0, 1) × (0, 1). W~H~ and the CR element spaces defined on the coarse mesh T fine mesh T~h~ is obtained from the coarse mesh T and V~h~ are chosen as the conforming linear element

H

and the fine mesh T~h~, respectively. The

H

by the regular refinement of uniform triangular mesh.

Since the coarse space W~H~ is the conforming linear finite element space defined on the coarse mesh T~H~, together with the theories of the error estimates of CR element and conforming linear element [8, 11], it is known that the following estimate holds when the mesh size h \< H, η~a~(W~H~) ≤ sup ∥T f − T~h~f∥~a,h~ + inf ∥T f − v~H~∥~a~

v ∈W f∈L<sup>2</sup>(Ω),∥f∥~b~=1 H H

≤ CH, where the constant C depends on shape of the meshes T Based on Theorems 3.1 and 3.2, the convergence results can be concluded with the following inequalities

(ℓ+1)

u¯~i,h~ − F~k,h~ u¯~i,h~

a,h 2ℓ

≤ C CH u¯~i,h~

16

and T~h~.

H

(1)

− F~k,h~u¯~i,h~

a,h

, i = 1 ,

- , k, (4.1)
