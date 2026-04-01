# Fibonacci sequence

The Fibonacci sequence is defined through the recurrence relation $F*n = F*(n-1) + F*(n-2)$. It can also be expressed in* closed form:_

$ F_n = round(1 / sqrt(5) phi.alt^n), quad phi.alt = (1 + sqrt(5)) / 2 $

if n \<= 2 { 1 } else { fib(n - 1) + fib(n - 2) } )

The first \#count numbers of the sequence are:

\#align(center, table( columns: count, ..nums.map(n =\> $F_\#n$), ..nums.map(n =\> str(fib(n))), ))
