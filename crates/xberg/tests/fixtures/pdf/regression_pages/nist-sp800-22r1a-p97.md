### 5.4 Data Input and Output of Empirical Results

### 5.4.1 Data Input

Data input may be supplied in one of two ways. If the user has a stand-alone program or hardware device which implements a RNG, the user may want to construct as many files of arbitrary length as desired. Files should contain binary sequences stored as either ASCII characters consisting of zeroes and ones, or as binary data where each byte contains eight bits worth of 0's and 1's. The NIST Statistical Test Suite can then independently examine these files.

In the event that storage space is a problem, the user may want to modify the reference implementation and plug-in their implementation of the PRNG under evaluation. The bit streams will be stored directly in the ***epsilon*** data structure, which contains binary sequences.

### 5.4.2 Output of Empirical Results

The output logs of empirical results will be stored in two files, *stats* and *results* , that correspond respectively to the computational information e.g., test statistics, intermediate parameters, and *P-values* for each statistical test applied to a data set.

If these files are not properly created, then it is most probably due to the inability to open the files for output. See Appendix C for further details.

### 5.4.3 Test Data Files

Six sample files have been created and are contained in the **data/** subdirectory. Four of these files

9

correspond to the Mathematica generated binary expansion of several classical numbers for over 1,000,000 bits. These files are *data.e* , *data.pi* , *data.sqrt2* , and *data.sqrt3* . The Mathematica program used in creating these files can be found in Appendix F. A fifth file, *data* . *sha1* , was constructed utilizing the SHA-1 hash function. The final sample file is one produced by a biased PRNG and is intended to fail certain statistical tests.

### 5.5 Program Layout

The test suite package has been decomposed into a series of modules which include the: statistical tests and (pseudo)random number generators, empirical results (hierarchical) directories, and data.

The three primary components of the NIST test suite are the statistical tests, the underlying mathematical software, and the pseudo random number generators under investigation. Other components include the source code library files, the data directory and the hierarchical directory ( **experiments** /) containing the sample data files and empirical result logs, respectively.

### 5.5.1 General Program

The NIST test suite contains fifteen tests which will be useful in studying and evaluating the binary sequences produced by random and pseudo random number generators. As in previous work in this field, statistical tests must be devised which, under some hypothesized distribution, employ a particular test statistic, such as the number of runs of ones or the number of times a pattern appears in a bit stream. The majority of the tests in the test suite either (1) examine the distribution of zeroes and ones in some

9

**Mathematica** , Stephen Wolfram's Computer Algebra System, http://www.wolfram.com/.
