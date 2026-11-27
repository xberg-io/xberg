at infinite temperature. In particular, we will employ discrete-time (blocked) cellular automaton circuits (CAC) to model the dynamics of these quantities in order to ascertain whether DFL can arise in this inherently classical setup. A further advantage of using CAC is that it allows accessing large system sizes and long evolution times.

**Cellular automaton circuits.—** Previous works have utilized CAC to probe the hydrodynamic behavior of (classical) systems with kinetic constraints and different kinds of unconventional conservation laws such as dipole-moment conservation [56–64]. The method is schematically depicted in Fig. 1a. Analogously to the quantum Hamiltonian in Eq. (1), we distinguish between matter sites (straight purple lines) of index j and gauge links (curly green lines) between sites, lying on a chain of L sites. The (classical) discrete spin $s_{j,j+1}$ representing the discretized local electric field on the link between sites j and j + 1 takes on values $s_{j,j+1}\in$ {−S, . . . , S}, while the matter occupation on site j is given by $\sigma_{j}\in\\{-1,+1\\}$, denoted in Fig. 1a as ◦ and

- , respectively. Consequently, the state of the system

at any time-step t is given by a string configuration $C(t)=(\sigma_{1},s_{1,2},\sigma_{2},...,s_{L-1,L},\sigma_{L},s_{L,1})$, explicitly assuming PBC. The dynamics is governed by local gates $P_{j,j+1}$ acting on pairs of neighboring sites j and j + 1 and their intermediate link, akin to the 3-local terms in the Hamiltonian (1). Using an analogous formulation to that in Ref. [64], we define the action of a local gate by two integers α and β, such that upon applying $P_{j,j+1}^{+}=\\{\alpha,-\beta\\}$ or its inverse $P_{j,j+1}^{-}=\\{-\alpha,\beta\\}$, the matter and gauge degrees of freedom are updated as $\sigma_{j}\rightarrow\sigma_{j}\pm\alpha,\sigma_{j+1}\rightarrow$ $\sigma_{j+1}\pm\alpha$ and $s_{j,j+1}\rightarrow s_{j,j+1}\mp\beta$, respectively. These updates are randomly applied among those for which $|\sigma_{j}\pm\alpha|=1,|\sigma_{j+1}\pm\alpha|$ = 1, and $|s_{j,j+1}\pm\beta|\leq S,$ with symmetric transition rates such that detailed balance is satisfied for the uniformly random ensemble over string configurations C. As a result, a local gate $P_{j,j+1}$ simply implements a random permutation between two allowed strings $C\rightarrow P_{j,j+1}C$. A (discrete) time-step is then given by a sequence of such non-overlapping gates acting on neighboring sites and the links in between as shown in Fig. 1a. The full time evolution follows from the application of several such layers of gates. In the following, we consider the case α = 2β = 2, to appropriately model the tunneling term of Hamiltonian (1). Furthermore, we define the "classical" Gauss's law as $G_{j}=(-1)^{j}s_{j-1,j}+s_{j,j+1}+(\sigma_{j}+1)/2$ at site j and its two neighboring links; cf. Eq. (2). One can then see that evaluating $G_{j}$ on a string C as given by ), agrees with ${}&#95;{+1}C$) for all $i,j=1,...,L.^{G_{j}(C}$This is trivially true for${}^{G_{j}(P_{i,i}}i6=j-1,j$. In the remainder cases, one finds`{}_{G_{j}}G_{j}(P_{i,i+1}C)=G_{j}(C)\pm(\beta-\alpha/`2), which is equal to (C) for α = 2β. Hence, the previously described CAC leaves the values of $G_{j}$ invariant for all j and at all

3

$$
S=1/2
$$

$$
S=1
$$

- (0) i

0.8

) σ~0~

|  | 1.0 |  |
| --- | --- | --- |
| (a) |  | (b) |
| L = 4 |  | L = 4 |
| L = 8 |  | L = 8 |
|  | 0.8 |  |
| L = 200 |  | L = 200 |
| M<sup>1</sup>(L = 8) |  | M (L = 6) |
|  | 0.6 0.4 |  |

2

hσ~0~(t

0.6

$$
10^{0}
$$

$$
10^{1}
$$

$$
10^{2}10^{0}
$$

$$
10^{1}
$$

$$
10^{2}
$$

t t (c)

i

0.4

0(∞) σ~0~(0)

0.2

hσ

L = 4 L = 200 L = 4 (M~S~) L = 6 L = 400 L = 6 (M~S~) 0.0 1 2 3 S

FIG. 2. (Color online). **Localized dynamics of the matter autocorrelator.**

Modeling of the dynamics of the matter autocorrelator for $(a) S = 1/2$ and $(b) S = 1$ for using cellular automaton circuits. The black horizontal line demarcates the corresponding Mazur bound (4). (c) Infinite-time value of the matter autocorrelator, showing a monotonic decrease with S, and convergence to the thermodynamic limit for system sizes L & 200 sites. Diamonds represent the finite-size Mazur's bounds obtained from Eq. (4).

times, leading to reducible classical dynamics whose sectors correspond to the gauge superselection sectors of the spin-S U(1) QLM (1). This motivates us to investigate whether DFL, well-established for the spin-S U(1) QLM, can also arise in this purely classical setting, in which case we can adjudge whether quantum interference effects are necessary for DFL to arise.

**Localized dynamics from CAC.—** We now focus on the dynamics as calculated through CAC. We prepare an initial string C(0) with a domain-wall structure in the matter degrees of freedom, i.e., `\sigma_{j}(0)=\pm`1 on the left (right) half of the chain, and a uniform random configuration of the electric field variables $s_{j,j+1}$. We then let it evolve and average the observables of interest A over N such different preparations hAi ≡ ${}^{P}{}&#95;{\\{C(t)\\}}A(C(t))/N.$ The imbalance is then computed as follows

$$
\mathcal{I}(t)=\frac{1}{L_{j=1}}^{X^{L}}h\sigma_{j}(0)\sigma_{j}(t)i.
$$

(3) The corresponding dynamics is shown in Fig. 1b for a system of $L = 500$ sites and for $N = 8000$ for various values of S. We see that the imbalance settles into a plateau of finite value at intermediate times that persists for all in-
