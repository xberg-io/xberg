HAL
archives-ouvertes.fr

# Bispindle in strongly connected digraphs with large chromatic number

Nathann Cohen, Frédéric Havet, William Lochet, Raul Lopes

## To cite this version:

Nathann Cohen, Frédéric Havet, William Lochet, Raul Lopes. Bispindle in strongly connected digraphs with large chromatic number. Electronic Notes in Discrete Mathematics, Elsevier, 2017, 62, pp.69 - 74. 10.1016/j.endm.2017.10.013 . hal-01634307

HAL Id: hal-01634307
https://hal.inria.fr/hal-01634307
Submitted on 13 Nov 2017

HAL is a multi-disciplinary open access archive for the deposit and dissemination of scientific research documents, whether they are published or not. The documents may come from teaching and research institutions in France or abroad, or from public or private research centers.

L'archive ouverte pluridisciplinaire HAL, est destinée au dépôt et à la diffusion de documents scientifiques de niveau recherche, publiés ou non, émanant des établissements d'enseignement et de recherche français ou étrangers, des laboratoires publics ou privés.

Bispindle in strongly connected digraphs with large chromatic number.

Nathann Cohen$\,{}^{1}$

CNRS, LRI, Univ. Paris Sud, Orsay, France

Frédéric Havet$\,{}^{2}$

Univ. Côte d’Azur, CNRS, I3S, INRIA, France

William Lochet$\,{}^{3}$

Univ. Côte d’Azur, CNRS, I3S, INRIA, and LIP, ENS Lyon, France

Raul Lopes$\,{}^{4}$

Departamento de Computação, Universidade Federal do Ceará, Fortaleza, Brazil

Abstract

A $(k_{1}+k_{2})$-bispindle is the union of $k_{1}$ $(x,y)$-dipaths and $k_{2}$ $(y,x)$-dipaths, all these dipaths being pairwise internally disjoint. Recently, Cohen et al. showed that for every $(2+0)$- bispindle $B$, there exists an integer $k$ such that every strongly connected digraph with chromatic number greater than $k$ contains a subdivision of $B$. We investigate generalisations of this result by first showing constructions of strongly connected digraphs with large chromatic number without any $(3+0)$-bispindle or $(2+2)$-bispindle. Then we show that for any $k$, there exists $\gamma_{k}$ such that every strongly connected digraph with chromatic number greater than $\gamma_{k}$ contains

a $(2+1)$-bispindle with the $(y,x)$-dipath and one of the $(x,y)$-dipaths of length at least $k$.

###### keywords:

Digraph, chromatic number, subdivision.

## 1 Introduction

Throughout this paper, the chromatic number of a digraph $D$, denoted by $\chi(D)$, is the chromatic number of its underlying graph. In a digraph $D$, a directed path, or dipath, is an oriented path where all the arcs are oriented form the initial vertex towards the terminal vertex. A $k$-spindle is the union of $k$ internally disjoint $(x,y)$-dipaths for some vertices $x$ and $y$. Vertex $x$ is said to be the tail of the spindle and $y$ its head. A $(k_{1}+k_{2})$-bispindle is the internally disjoint union of a $k_{1}$-spindle with tail $x$ and head $y$ and a $k_{2}$-spindle with tail $y$ and head $x$. In other words, it is the union of $k_{1}$ $(x,y)$-dipaths and $k_{2}$ $(y,x)$-dipaths, all of these dipaths being pairwise internally disjoint.

A classical result due to Gallai, Hasse, Roy and Vitaver is the following.

###### Theorem 1.1 (Gallai *[8]*, Hasse *[9]*, Roy *[11]*, Vitaver *[12]*)

If $\chi(D)\geq k$, then $D$ contains a dipath of length $k-1$.

This raises the question of which digraphs are subdigraphs of all digraphs with large chromatic number.

A classical theorem by Erdős *[6]* implies that if $H$ is a digraph containing a cycle, there exist digraphs with arbitrarily high chromatic number with no subdigraph isomorphic to $H$. Thus the only possible candidates to generalise Theorem 1.1 are the oriented trees that are orientations of trees. Burr*[3]* proved that every $(k-1)^{2}$-chromatic digraph contains every oriented tree of order $k$ and conjectured an upper bound of $2k-2$. The best known upper bound, due to Addario-Berry et al. *[1]*, is in $(k/2)^{2}$.

However the following celebrated theorem of Bondy shows that the story does not stop there.

###### Theorem 1.2 (Bondy *[2]*)

Every strongly connected digraph with chromatic number at least $k$ contains a directed cycle of length at least $k$.

##

The strong connectivity assumption is indeed necessary, as transitive tournaments contain no directed cycle but can have arbitrarily high chromatic number.

Observe that a directed cycle of length at least $k$ can be seen as a subdivision of $\vec{C}_{k}$, the directed cycle of length $k$. Recall that a subdivision of a digraph $F$ is a digraph that can be obtained from $F$ by replacing each arc $uv$ by a dipath from $u$ to $v$.

###### Conjecture 1.3 (Cohen et al. *[5]*)

For every cycle $C$, there exists a constant $f(C)$ such that every strongly connected digraph with chromatic number at least $f(C)$ contains a subdivision of $C$.

The strong connectivity assumption is also necessary in Conjecture 1.3 as shown by Cohen et al. in *[5]*. In the same paper, Conjecture 1.3 was confirmed for cycles with two blocks (i.e. maximal subdipaths of the cycle) and the antidirected cycle of length $4$. More precisely, denoting by $C(k,\ell)$ the cycle on two blocks, one of length $k$ and the other of length $\ell$, Cohen et al. *[5]* proved the following.

###### Theorem 1.4

Every strongly connected digraph with chromatic number at least $O((k+\ell)^{4})$ contains a subdivision of $C(k,\ell)$.

The bound has recently been improved to $O((k+\ell)^{2})$ by Kim et al. *[10]*.

A subdivision of $C(k,\ell)$ can be seen as a $2$-spindle made of two internally disjoint dipaths, one of length at least $k$ and one of length at least $\ell$. In this paper, we generalize this and study the existence of subdivision of spindles and bispindles in strongly connected digraphs with large chromatic number. Our first result is to give constructions for the following theorem:

###### Theorem 1.5

For every integer $k$, there exists a strongly connected digraph $D$ with $\chi(D)>k$ that contains no $3$-spindle and no $(2+2)$-bispindle.

Therefore, the most we can expect in all strongly connected digraphs with large chromatic number are $(2+1)$-bispindle. Let $B(k_{1},k_{2};k_{3})$ denote the $(2+1)$-bispindle formed by three internally disjoint paths between two vertices $x,y$, two $(x,y)$-dipaths, one of size $k_{1}$, the other of size $k_{2}$, and one $(y,x)$-dipath of size $k_{3}$. We conjecture the following.

###### Conjecture 1.6

There is a function $g:\mathbb{N}^{3}\to\mathbb{N}$ such that every strongly connected digraph with chromatic number at least $g(k_{1},k_{2},k_{3})$ contains a subdivision of $B(k_{1},k_{2};k_{3})$.

As an evidence, we prove the following theorem:

###### Theorem 1.7

For every positive integer $k$, there is a constant $\gamma_{k}$ such that every strongly connected digraph witch chromatic number greater than $\gamma_{k}$ contains a subdivision of $B(k,1;k)$.

The value of $\gamma_{k}$ is the above theorem is huge, and certainly not best possible. We get a better bound for subdivision of $B(k,1;k)$.

###### Theorem 1.8

Let $k\geq 3$ be an integer and let $D$ be a strong digraph. If $\chi(D)>(2k-2)(2k-3)$, then $D$ contains a subdivision of $B(k,1;1)$.

## 2 Proof of Theorem 1.7

We prove Theorem 1.7 by the contrapositive. We consider a digraph $D$ without any subdivision of $B(k,1;k)$. We shall prove that $\chi(D)\leq\gamma_{k}$.

The general idea is to use the following easy lemma.

###### Lemma 2.1

Let $D$ be a digraph, $D_{1}\ldots D_{l}$ be disjoint subdigraphs of $D$ and $D^{\prime}$ the digraph obtained by contracting each $D_{i}$ into one vertex $d_{i}$. Then $\chi(D)\leq\chi(D^{\prime})\cdot\max\{\chi(D_{i})\mid 1\leq i\leq l\}$.

The key is to find appropriate subdigraphs $D_{i}$. To do so, we consider some particular collections of directed cycles : a collection $\mathcal{C}$ of directed cycles is $k$-suitable if all cycles of $\mathcal{C}$ have length at least $8k$, and any two distinct cycles $C_{i},C_{j}\in\mathcal{C}$ intersect on a subpath of order at most $k$. A component of $\mathcal{C}$ is a connected component of the underlying graph of the digraph $\bigcup\mathcal{C}$ which is the union of cycles of $\mathcal{C}$.

Consider $\mathcal{C}$ be a maximal $k$-suitable collection of cycles in $D$. Let $D^{\prime}$ be the digraph obtained by contracting every strong component $S$ of $\bigcup\mathcal{C}$ (which is $\bigcup\mathcal{S}$ for some component $\mathcal{S}$ of $\mathcal{C}$) into one vertex. For each connected component $\mathcal{S}_{i}$ we call $s_{i}$ the new vertex created. To apply Lemma 2.1, we shall prove in the next two lemmas that for every component $\mathcal{S}$ of $\mathcal{C}$, the digraph $D[\mathcal{S}]$ induced by $D$ on the vertices of $\bigcup\mathcal{S}$ has bounded chromatic number and that $\chi(D^{\prime})\leq 8k$.

###### Lemma 2.2

Let $\mathcal{C}$ be a $k$-suitable collection of directed cycles in a $B(k,1;k)$-free digraph $D$. There exists a constant $\beta_{k}$ such that, for every component $\mathcal{S}$ of $\mathcal{C}$, we have $\chi(D[\mathcal{S}])\leq\beta_{k}$.

Sketch of proof: We first consider $\bigcup\mathcal{S}$ which is a subdigraph of $D[\mathcal{S}]$. We prove by induction on the number of cycles in $\mathcal{S}$ that this digraph admits a proper colouring $\phi$ with $\alpha_{k}=2\cdot(6k^{2})^{3k}+14k$ colours satisfying the following

additional property, called rainbow property: the vertices of each subpath of length at most $7k$ of each cycle of $\mathcal{S}$ get different colours.

We then define a sort of Breadth-First-Search for $\mathcal{S}$. Let $C_{0}$ be a cycle of $\mathcal{S}$ and set $L_{0}=\{C_{0}\}$. We build the levels $L_{i}$ inductively until all cycles of $\mathcal{S}$ are put in a level: $L_{i+1}$ consists of every cycle $C_{l}$ not in $\bigcup_{j\leq i}L_{j}$ such that there exists a cycle in $L_{i}$ intersecting $C_{l}$. For every $C_{l}\in L_{i+1}$, we choose one of the cycles $L_{i}$ intersecting it to be its father. For a vertex $x$ of $\bigcup\mathcal{S}$, we say that $x$ belongs to level $L_{i}$ if $i$ is the smallest integer such that there exists a cycle in $L_{i}$ containing $x$.

We partition the arc set of $D[\mathcal{S}]$ in $(A_{0},A_{1},A_{2})$, where

- $A_{0}$ is the set of arcs of $D[\mathcal{S}]$ which ends belong to the same level, and
- $A_{1}$ is the set of arcs of $D[\mathcal{S}]$ which ends belong to different levels $i$ and $j$ with $|i-j|<k$.
- $A_{2}$ is the set of arcs of $D[\mathcal{S}]$ which ends belong to different levels $i$ and $j$ with $|i-j|\geq k$.

For $i\in\{0,1,2\}$, let $D_{i}$ be the spanning subdigraph of $D[\mathcal{S}]$ with arc set $A_{i}$. It is well-known that $\chi(D[\mathcal{S}])\leq\chi(D_{0})\times\chi(D_{1})\times\chi(D_{2})$.

Clearly, $\chi(D_{1})\leq k$, and we show that $\chi(D_{2})\leq 4k^{2}+2$. To bound $\chi(D_{0})$ we partition the vertex set according to the above-mention colouring $\phi$ of $\bigcup\mathcal{S}$. Using the rainbow property, we prove that the subdigraph of $D_{0}$ induced by the vertices of colour $c$ has chromatic number at most $2\cdot(4k)^{4k}+1$ for all colour $c$. Hence $\chi(D_{0})\leq(2\cdot(4k)^{4k}+1)\alpha_{k}$. This gives the result for $\beta_{k}=k(4k^{2}+2)(2\cdot(4k)^{4k}+1)\alpha_{k}$. ∎

###### Lemma 2.3

$\chi(D^{\prime})\leq 8k$.

Proof. First note that since $D$ is strongly connected so is $D^{\prime}$.

Suppose for a contradiction that $\chi(D^{\prime})>8k$. By Theorem 1.2, there exists a directed cycle $C^{\prime}=(x_{1},x_{2},\ldots,x_{l},x_{1})$ of length at least $8k$. For each vertex $x_{j}$ that corresponds to a $\mathcal{S}_{i}$ in $D$, the arc $x_{j-1}x_{j}$ corresponds in $D$ to an arc whose head is a vertex $p_{i}$ of $\mathcal{S}_{i}$ and the arc $x_{j}x_{j+1}$ corresponds to an arc whose tail is a vertex $l_{i}$ of $\mathcal{S}_{i}$. Let $P_{j}$ be the dipath from $p_{i}$ to $l_{i}$ in $\bigcup\mathcal{C}$. Note that this path intersects the elements of $\mathcal{S}_{i}$ only along a subdipath. Let $C$ be the cycle obtained from $C^{\prime}$ where we replace all contracted vertices $x_{j}$ by the path $P_{j}$. First note that $C$ has length at least $8k$. Moreover, a cycle of $\mathcal{C}$ can intersect $C$ only along one $P_{j}$, because they all correspond to different strong components of $\bigcup\mathcal{C}$. Thus $C$ intersects each cycle of $\mathcal{C}$ on a subdipath. Moreover this subdipath has length smaller than $k$ for otherwise $D$ would contain a subdivision of $B(k,1;k)$. So $C$ is a directed cycle of length at leas

$8k$ which intersects every cycle of $\mathcal{C}$ along a subdipath of length less than $k$. This contradicts the maximality of $\mathcal{C}$. ∎

Using Lemma 2.1 with Claim 2.3 and Lemma 2.2, we get that $\chi(D)\leq 8k\cdot\beta_{k}$. This proves Theorem 1.7 for $\gamma_{k}=8k\cdot\beta_{k}$.

## References

- [1] L. Addario-Berry, F. Havet, C. L. Sales, B. A. Reed, and S. Thomassé. Oriented trees in digraphs. Discrete Mathematics, 313 (8): 967–974, 2013.
- [2] J. A. Bondy, Disconnected orientations and a conjecture of Las Vergnas, J. London Math. Soc. (2), 14 (2): 277–282, (1976).
- [3] S. A. Burr. Subtrees of directed graphs and hypergraphs. In Proceedings of the 11th Southeastern Conference on Combinatorics, Graph theory and Computing, pages 227–239, Boca Raton - FL, 1980. Florida Atlantic University.
- [4] S. A. Burr, Antidirected subtrees of directed graphs. Canad. Math. Bull. 25 : 1982 119–120, 1982
- [5] N. Cohen, F. Havet, W. Lochet, and N. Nisse. Subdivisions of oriented cycles in digraphs with large chromatic number. arXiv:1605.07762
- [6] P. Erdős. Graph theory and probability. Canad. J. Math., 11:34–38, 1959.
- [7] P. Erdős and A. Hajnal. On chromatic number of graphs and set-systems. Acta Mathematica Academiae Scientiarum Hungarica, 17(1-2):61–99, 1966.
- [8] T. Gallai. On directed paths and circuits. In Theory of Graphs (Proc. Colloq. Titany, 1966), pages 115–118. Academic Press, New York, 1968.
- [9] M. Hasse. Zur algebraischen bergründ der graphentheorie I. Math. Nachr., 28: 275–290, 1964.
- [10] R. Kim, SJ. Kim, J. Ma; B. Park Cycles with two blocks in $k$-chromatic digraphs arXiv:1610.05839
- [11] B. Roy. Nombre chromatique et plus longs chemins d’un graphe. Rev. Francaise Informat. Recherche Opérationnelle, 1 (5): 129–132, 1967.
- [12] L. M. Vitaver. Determination of minimal coloring of vertices of a graph by means of boolean powers of the incidence matrix. Doklady Akademii Nauk SSSR, 147: 758–759, 1962.
