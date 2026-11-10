attractive choice for simulation when applicable. Correspondingly, exciting opportunities open up when new interesting physical models are found which can be simulated using this method [46–48]. Recently, there has been intense experimental development to study the physics of confinement and quantum spin liquids [49–55] using tools of quantum simulation and computation. The microscopic models used to capture the physics involve fermions interacting with

- (Abelian) gauge fields. In this Letter, we develop meron cluster algorithms for a class of experimentally relevant models [56, 57], enabling a robust elucidation of their phase diagrams. We also introduce new classes of Z~2~ and U(1) multi-flavored gauge-fermion theories, which might be realized in cold-atom setups and also be further studied using Monte Carlo techniques. Notably, the U(1) family of these models seems to be one of the few families that falls outside the class of models known to be simulable by auxiliary-field methods, as are [23, 58, 59]. Moreover, the worldline nature of the method makes it easily employed to study the corresponding phases in these theories in higher spatial dimensions, and the resulting physically-relevant configurations are promising inputs for machine learning algorithms.

Models.– We start with the half-filled t-V model–a spinless fermionic Hamiltonian involving only the most local interactions, X t 1 1 † † H = − (c~x~c~y~ + c~y~c~x~) + V (n~x~ − )(n~y~ − ) (1) 2 2 2

⟨xy⟩ †

Here ⟨xy⟩ are the nearest neighbor site pairs, c , c are the creation and annihilation operators respectively, and the repulsive interaction V is given in terms of the number

†

operator n = c c. It is simulable by meron clusters for V ≥ 2t [45, 48]. In this Letter, we extend the meron cluster method to physically interesting Hamiltonians involving gauge fields, which are lower-dimensional versions of quantum electrodynamics (QED) [60, 61]. The Z~2~- and U(1)-gauge symmetric families are given by:

N~f~

XY

(g) (g) (g),des

H~Nf~ = − H + H (2) ~⟨~xy⟩,f ⟨xy⟩,f

⟨xy⟩ f=1

The label g ∈ {U(1), Z~2~} is the gauge symmetry, with

† 1 † 1

H~⟨~ <sup>Z</sup>~2~ xy⟩,f = t c s~xy,f~c~y,f~ + c~y,f~s~xy,f~c~x,f~ x,f (3)

U(1) † + † −

H = t c s c~y,f~ + c s c~x,f~ . ⟨xy⟩,f x,f xy,f y,f xy,f The hopping of spinless fermions between the nearest neighbors ⟨xy⟩ are now governed by the presence of gauge

k

fields, represented by spin-1/2 operators, s~xy~, on the bond. Fig. 1 illustrates the model degrees of freedom.

(g),des

Then H is a designer term [62] that makes the mod⟨xy⟩,f

2

fermion hole spin-1/2 up spin-1/2 down FIG. 1: Example fermion occupations and bond variables for the theories on a square spatial lattice with N~f~ = 1. els simulable by the meron algorithm, 1 1 t H~⟨~ <sup>Z</sup>~2~ xy ,des ⟩,f = −2t n~x,f~ − n~y,f~ − + 2 2 2 1 1 U(1),des

- H = −t n (4) ⟨xy⟩,f x,f − n~y,f~ − 2 2 t

3

− ts~xy,f~ (n~y,f~ − n~x,f~) + 4 These terms serve a similar role as the V = 2t term in the meron cluster algorithm applied to the t-V model [45], and similarly an additional particle-hole symmetric V ≥ 0 term can be added to the models here in a signproblem-free way. In the Z~2~ gauge theory, the gauge field

s<sup>1</sup> = σ<sup>1</sup>/2 couples to fermions, and the local Z~2~ symmetry is manifest via the commutation [Q~x~, H<sup>Z</sup>~2~] = 0,

P

Q

n~x,f~ 3 3 f

where Q~x~ = (−1) s s , αˆ are the f,αˆ x,x+ˆα,f x−α,x,fˆ unit vectors in a d-dimensional square lattice. For the U(1) theory, the unitary operator V~U~(1), which commutes Q

U(1)

with H is given by V~U~(1) = e<sup>iθxGx</sup>, with G~x~ = h x i P P

3 3

n~x,f~ − s − s + ((−1)<sup>x</sup> − 1)/2 .

f αˆ x,x+ˆα,f x−α,x,fˆ

In the terminology of gauge fields, our microscopic models are quantum link models [63], which realize the continuous gauge invariance using finite-dimensional quantum degrees of freedom. The identification with usual gauge

\+ † − 3

field operators is U = s , U = s , E = s~xy,f~ xy,f . xy,f xy,f xy,f We note that a straightforward application of the meron idea necessitates the introduction of an equivalent flavor index for gauge links as fermion flavors. Naively, the total Gauss law can be expressed through a product (Z~2~) or sum (U(1)) of the Gauss law of individual flavors degrees of freedom, and the resulting theories have

⊗N~f~

Z~2~ and U(1)⊗N~f~ gauge symmetry. However, flavored gauge-interactions can also be turned on in the U(1) model (as explained in the Supplementary Material), X

U(1) U(1) 3 3

H~Nf~=2 →H~Nf~=2 + J s~xy,~1s~xy,~2, (5)

⟨xy⟩

or through a Hubbard-U interaction for both Z~2~ and U(1)-symmetric models [48]. These additions directly
