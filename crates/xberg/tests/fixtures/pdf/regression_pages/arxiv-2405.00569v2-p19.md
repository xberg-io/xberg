Central compact schemes for third derivatives 19

```
0
-1.91
0
-1.91
-0.2 -1.92 -0.2 -1.92
-0.4 -0.4
-1.93 -1.93
-0.6 -0.6
-1.94 -1.94
-0.8 -0.8
-1.95 -1.95
-1 -1
```

```
u(x,t)
u(x,t)
u(x,t)
```

**-1.96 u(x,t)**

```
-1.96
-1.2 -1.2
-1.4 -1.97 -1.4 -1.97
-1.6
Exact-CNCS -1.98
Exact-CNCS
F-0
F-0
-1.8 F-8
F-8
F-10 F-10 F-10 F-10
-1.99 -1.99
-2 F-12 F-12 -2 F-12 F-12
-10 -5 0 5 10 0.35 0.4 0.45 0.5 0.55 0.6
x x
```

| -1.6 Exact-CNCS | Exact-CNCS -1.98 |
| --- | --- |
| F-0 | F-0 |
| -1.8 | F-8 |

| -2 |  |  |  | -2 |
| --- | --- | --- | --- | --- |
| 15 0.3 | 0.35 0.4 0.45 0.5 0.55 | -10 0.6 | -5 0 5 10 | 15 0.3 |
|  | x |  | x |  |

- (a) TDCNCS, filtered ev- (b) Zoom in near smooth (c) TDCNCS, filtered ev- (d) Zoom in near smooth

ery 2 time steps extrema ery 10 time steps extrema

```
0
-1.91
0
-1.91
-0.2 -1.92 -0.2 -1.92
-0.4 -0.4
-1.93 -1.93
-0.6 -0.6
-1.94 -1.94
-0.8 -0.8
-1.95 -1.95
-1 -1
```

```
u(x,t)
u(x,t)
u(x,t)
```

**-1.96 u(x,t)**

```
-1.96
-1.2 -1.2
-1.4 -1.97 -1.4 -1.97
-1.6
Exact-CCS -1.98
Exact-CCS
F-0
F-0
-1.8 F-8
F-8
F-10 F-10 F-10 F-10
-1.99 -1.99
-2 F-12 F-12 -2 F-12 F-12
-2 -2
-10 -5 0 5 10 15 0.3 0.35 0.4 0.45 0.5 0.55 0.6 -10 -5 0 5 10 15 0.3 0.35 0.4 0.45 0.5 0.55 0.6
x x x x
```

| -1.6 Exact-CCS | Exact-CCS -1.98 |
| --- | --- |
| F-0 | F-0 |
| -1.8 | F-8 |

- (e) TDCCS, filtered every (f) Zoom in near smooth (g) TDCCS, filtered every (h) Zoom in near smooth 2 time steps extrema 10 time steps extrema

Figure 9. Analysis of filter at N = 40, t = 0.1 for Example 7.2.

−4

where ϵ = 10 . The solution is simulated over the interval t ∈ [0, 4] using periodic boundary conditions,

employing eighth-order TDCNCS and TDCCS schemes with a grid size of N = 150 and time step ∆t = 0.5∆x<sup>2</sup> as in [11]. Figure (12) illustrates the solution at time instances t = 0, 1, 2, and its contour plot up to t = 4. Notably, TDCNCS exhibits small-scale numerical oscillations, whereas TDCCS produces a smooth numerical solution. To address the presence of high-frequency oscillations, we apply a 12th-order periodic filter with α = 0.4 every 20 steps for TDCNCS and every 50 steps for TDCCS (time steps = 20000). Our experiments indicate that filtering every 50 steps does not sufficiently eliminate the small oscillations in TDCNCS.

**Example 7.4.** In this example, we investigate the zero dispersion limit of conservation laws, specifically the

KdV equation (7.3) with the continuous initial condition 1 (7.7) u~0~(x) = 2 + sin(2πx), x ∈ [0, 1], 2

with periodic boundary conditions. Theoretical and numerical analysis concerning the limit as ϵ → 0<sup>+</sup> are available in [ **?** ] and [64]. The primary goal is assessing our numerical method capability to resolve small-scale solution structures in this limit when ϵ is small. To address this, we compute solutions for t = 0.5 with

−4 −7

ϵ = 10 , 10<sup>−5</sup>, 10<sup>−6</sup>, and 10 using eighth-order TDCNCS and TDCCS, with a step size ∆t = ∆x<sup>2</sup> as in [11]. Since the exact solution is not known for this problem, we utilize a reference solution acquired through

−4

a significantly higher number of grid points (N = 1000) using TDCNCS. For ϵ = 10 (with N = 100)

−5

and ϵ = 10 (with N = 200), Figures (13)(a) and (13)(b) depict the solutions obtained by TDCNCS (in

−6

red), TDCCS (in blue), in comparison with the reference solution (in black). Similarly, for ϵ = 10 (with

−7

N = 800) and ϵ = 10 (with N = 1600) using both TDCNCS and TDCCS, Figures (13)(c)-(f) showcase these solutions, confirmed as "converged", exhibiting physical oscillations typical in dispersive limits [64]. Moreover, inadequate mesh refinement results in failure to achieve a converged solution. For instance, when

−6

ϵ = 10 , we observed that numerical solutions obtained with 200, 300, 400, and even 600 uniform cells did
