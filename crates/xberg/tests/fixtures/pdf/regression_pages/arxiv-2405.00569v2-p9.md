Central compact schemes for third derivatives 9 2

W(ω) is [1 + 2α cos(ω) + 2β cos(2ω)] . In this work, we opt for r = 1 to establish the finite difference coefficients. (3.9) Z

rπ ω 2b 3ω 2c 5ω 2

2a[8 sin( ) − 4 sin(ω)] + [12 sin(ω) − 8 sin( )] + [20 sin(ω) − 8 sin( )] 2 5 2 35 2

min E = −ω<sup>3</sup> W(ω)dω.

a,b,c,α,β

1 + 2α cos(ω) + 2β cos(2ω)

0

The LS method solves the optimization problem by setting the derivatives of the misfit function for the unknown parameters to zero and then solving the resulting linear algebraic system:

- Fourth-order tri-diagonal TDCCS (

∂E

= 0,

∂a

1 + 2α + 2β = a + b + c, b = 0, c = 0, β = 0.

- Sixth-order tri-diagonal TDCCS (

∂E ∂E

= 0, = 0,

∂a ∂b

1 + 2α + 2β = a + b + c, c = 0, β = 0.

- Eighth-order tri-diagonal TDCCS (

∂E ∂E ∂E

= 0, = 0, = 0,

∂a ∂b ∂c

1 + 2α + 2β = a + b + c, β = 0.

- Tenth-order penta-diagonal TDCCS 

∂E ∂E

<sup>∂</sup>E = 0, = 0, = 0, ~∂a~

∂b ∂c

1 + 2α + 2β = a + b + c, 

a 13b 29c

α + 2<sup>2</sup>β = + + . 16 80 80 Note that we denote TDCCS with the least squares-based coefficients as TDCCS-LS.

4. Fourier analysis of the errors

The main incentive behind developing TDCCS schemes is to precisely resolve small scales in multiscale physical problems. Consequently, Fourier analysis is conducted on these optimal schemes to evaluate their spectral characteristics. In this section, we investigate the dispersion and dissipation properties of TDCCS through Fourier analysis. TDCCS, being a central difference, eliminates numerically dissipative errors. The Fourier transformation, a frequently employed tool in finite difference scheme analysis, is applied to equation (2.5), and by utilizing Euler's formula, the modified wavenumber ω

ω

2a[8 sin( ) − 4 sin(ω)] +

′′′ 2

(4.1) ω~TDCCS~ = Here, ω represents the scaled wavenumber with ω = kh, and ω

′′′

of TDCCS can be derived. It is:

2b 3ω 2c 5ω

[12 sin(ω) − 8 sin( )] + [20 sin(ω) − 8 sin( )]

5 2 35 2

. 1 + 2α cos(ω) + 2β cos(2ω)

′′′

represents the scaled modified wavenumber.

Figure 3 presents plots comparing modified wavenumber versus wavenumber for third derivative approximations using different methods: (a) the Taylor expansion-based method (TDCCS-TE), (b) the least square optimization-based method (TDCCS-LS), and (c) the tenth-order pentadiagonal compact interpolation (TDCCS-CI). The schemes encompass various orders and types, including (i) fourth-order explicit scheme (E4), (ii) sixth-order explicit scheme (E6), (iii) fourth-order tridiagonal scheme (T4), (iv) sixth-order tridiagonal scheme (T6), (v) eighth-order tridiagonal scheme (T8), (vi) sixth-order pentadiagonal scheme (P6), (vii) eighth-order pentadiagonal scheme (P8), and (viii) tenth-order pentadiagonal scheme (P10). The resolutions achieved by TDCCS-TE and TDCCS-LS surpass those of TDCCCS and TDCNCS. The difference between the modified wavenumber of TDCCS-TE and TDCCS-LS with the exact wavenumber is minimal. In particular, for pentadiagonal schemes, the wave number of TDCCS cannot be distinguished from the exact
