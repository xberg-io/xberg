Central compact schemes for third derivatives 9

W(ω) is [1 + 2$\alpha\cos(\omega)+2\beta\cos(2\omega)&#93;^{2}.$ In this work, we opt for r = 1 to establish the finite difference coefficients. (3.9)

$$
{}^{Z}{}^{r\pi}2a[8\sin(\frac{\omega}{2})-4\sin(\omega)]+\frac{2b}{5}[12\sin(\omega)-8\sin(\frac{3\omega}{2})]+\frac{2c}{35}[20\sin(\omega)-8\sin(\frac{5\omega}{2})]_{-\omega^{3}}
$$

min E =

$$
{}^{2}W(\omega)d\omega.
$$

a,b,c,α,β

$$
1+2\alpha\cos(\omega)+2\beta\cos(2\omega)
$$

0

The LS method solves the optimization problem by setting the derivatives of the misfit function for the unknown parameters to zero and then solving the resulting linear algebraic system:

- Fourth-order tri-diagonal TDCCS

${}^{(}\frac{\partial E}{\partial a}=0,$

$$
1+2\alpha+2\beta=a+b+c,b=0,c=0,\beta=0.
$$

- Sixth-order tri-diagonal TDCCS

${}^{(}\frac{\partial E}{\partial a}=0,\frac{\partial E}{\partial b}=0,$

$$
1+2\alpha+2\beta=a+b+c,c=0,\beta=0.
$$

- Eighth-order tri-diagonal TDCCS

${}^{(}\frac{\partial E}{\partial a}=0,\frac{\partial E}{\partial b}=0,\frac{\partial E}{\partial c}=0,$

$$
1+2\alpha+2\beta=a+b+c,\beta=0.
$$

- Tenth-order penta-diagonal TDCCS

$\frac{\partial E}{\partial a}=0,\frac{\partial E}{\partial b}=0,\frac{\partial E}{\partial c}=0,$ 1 + 2α + 2β = a + b + c, 

a 13b 29c

α + 2<sup>2</sup>β = + + . 16 80 80 Note that we denote TDCCS with the least squares-based coefficients as TDCCS-LS.

4. Fourier analysis of the errors

The main incentive behind developing TDCCS schemes is to precisely resolve small scales in multiscale physical problems. Consequently, Fourier analysis is conducted on these optimal schemes to evaluate their spectral characteristics. In this section, we investigate the dispersion and dissipation properties of TDCCS through Fourier analysis. TDCCS, being a central difference, eliminates numerically dissipative errors. The Fourier transformation, a frequently employed tool in finite difference scheme analysis, is applied to equation (2.5), and by utilizing Euler's formula, the modified wavenumber $\omega'''$ of TDCCS can be derived. It is:

$$
2a[8\sin(\frac{\omega}{2})-4\sin(\omega)]+\frac{2b}{5}[12\sin(\omega)-8\sin(\frac{3\omega}{2})]+\frac{2c}{35}[20\sin(\omega)-8\sin(\frac{5\omega}{2})]_{.}
$$

(4.1) ω<sup>′′′</sup> ~TDCCS~ =

$$
1+2\alpha\cos(\omega)+2\beta\cos(2\omega)
$$

Here, ω represents the scaled wavenumber with ω = kh, and $\omega'''$ represents the scaled modified wavenumber.

Figure 3 presents plots comparing modified wavenumber versus wavenumber for third derivative approximations using different methods: (a) the Taylor expansion-based method (TDCCS-TE), (b) the least square optimization-based method (TDCCS-LS), and (c) the tenth-order pentadiagonal compact interpolation (TDCCS-CI). The schemes encompass various orders and types, including (i) fourth-order explicit scheme (E4), (ii) sixth-order explicit scheme (E6), (iii) fourth-order tridiagonal scheme (T4), (iv) sixth-order tridiagonal scheme (T6), (v) eighth-order tridiagonal scheme (T8), (vi) sixth-order pentadiagonal scheme (P6), (vii) eighth-order pentadiagonal scheme (P8), and (viii) tenth-order pentadiagonal scheme (P10). The resolutions achieved by TDCCS-TE and TDCCS-LS surpass those of TDCCCS and TDCNCS. The difference between the modified wavenumber of TDCCS-TE and TDCCS-LS with the exact wavenumber is minimal. In particular, for pentadiagonal schemes, the wave number of TDCCS cannot be distinguished from the exact
