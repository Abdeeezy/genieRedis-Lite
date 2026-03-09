
### redis-benchmark -p 6379 -t set,get -n 100000 -c 50 
    - testing SET and GET 
    - 100000 times
    - 50 concurrent parallel clients

The highlights:
    - SET: ~86k ops/sec, p50 0.33ms, p99 0.44ms
    - GET: ~106k ops/sec, p50 0.24ms, p99 0.42ms
    - 99.9%+ of requests under 1ms


```
WARNING: Could not fetch server CONFIG
====== SET ======                                                   
  100000 requests completed in 1.16 seconds
  50 parallel clients
  3 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.047 milliseconds (cumulative count 1)
50.000% <= 0.327 milliseconds (cumulative count 55097)
75.000% <= 0.351 milliseconds (cumulative count 77803)
87.500% <= 0.367 milliseconds (cumulative count 88649)
93.750% <= 0.383 milliseconds (cumulative count 94623)
96.875% <= 0.399 milliseconds (cumulative count 97385)
98.438% <= 0.423 milliseconds (cumulative count 98716)
99.219% <= 0.455 milliseconds (cumulative count 99257)
99.609% <= 0.535 milliseconds (cumulative count 99613)
99.805% <= 0.807 milliseconds (cumulative count 99808)
99.902% <= 1.159 milliseconds (cumulative count 99905)
99.951% <= 1.383 milliseconds (cumulative count 99953)
99.976% <= 1.479 milliseconds (cumulative count 99977)
99.988% <= 1.543 milliseconds (cumulative count 99988)
99.994% <= 1.575 milliseconds (cumulative count 99995)
99.997% <= 1.599 milliseconds (cumulative count 99997)
99.998% <= 3.879 milliseconds (cumulative count 99999)
99.999% <= 4.703 milliseconds (cumulative count 100000)
100.000% <= 4.703 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
0.006% <= 0.103 milliseconds (cumulative count 6)
0.369% <= 0.207 milliseconds (cumulative count 369)
33.794% <= 0.303 milliseconds (cumulative count 33794)
98.035% <= 0.407 milliseconds (cumulative count 98035)
99.532% <= 0.503 milliseconds (cumulative count 99532)
99.687% <= 0.607 milliseconds (cumulative count 99687)
99.755% <= 0.703 milliseconds (cumulative count 99755)
99.808% <= 0.807 milliseconds (cumulative count 99808)
99.836% <= 0.903 milliseconds (cumulative count 99836)
99.854% <= 1.007 milliseconds (cumulative count 99854)
99.893% <= 1.103 milliseconds (cumulative count 99893)
99.912% <= 1.207 milliseconds (cumulative count 99912)
99.935% <= 1.303 milliseconds (cumulative count 99935)
99.961% <= 1.407 milliseconds (cumulative count 99961)
99.980% <= 1.503 milliseconds (cumulative count 99980)
99.997% <= 1.607 milliseconds (cumulative count 99997)
99.998% <= 2.007 milliseconds (cumulative count 99998)
99.999% <= 4.103 milliseconds (cumulative count 99999)
100.000% <= 5.103 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 86132.64 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        0.321     0.040     0.327     0.391     0.439     4.703
====== GET ======                                                     
  100000 requests completed in 0.94 seconds
  50 parallel clients
  3 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.063 milliseconds (cumulative count 3)
50.000% <= 0.239 milliseconds (cumulative count 56229)
75.000% <= 0.295 milliseconds (cumulative count 77547)
87.500% <= 0.327 milliseconds (cumulative count 87887)
93.750% <= 0.359 milliseconds (cumulative count 95107)
96.875% <= 0.375 milliseconds (cumulative count 96976)
98.438% <= 0.399 milliseconds (cumulative count 98478)
99.219% <= 0.439 milliseconds (cumulative count 99242)
99.609% <= 0.503 milliseconds (cumulative count 99632)
99.805% <= 0.623 milliseconds (cumulative count 99805)
99.902% <= 0.815 milliseconds (cumulative count 99904)
99.951% <= 0.999 milliseconds (cumulative count 99952)
99.976% <= 1.079 milliseconds (cumulative count 99978)
99.988% <= 1.135 milliseconds (cumulative count 99989)
99.994% <= 1.263 milliseconds (cumulative count 99994)
99.997% <= 1.463 milliseconds (cumulative count 99997)
99.998% <= 1.791 milliseconds (cumulative count 99999)
99.999% <= 2.247 milliseconds (cumulative count 100000)
100.000% <= 2.247 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
0.015% <= 0.103 milliseconds (cumulative count 15)
6.706% <= 0.207 milliseconds (cumulative count 6706)
80.230% <= 0.303 milliseconds (cumulative count 80230)
98.694% <= 0.407 milliseconds (cumulative count 98694)
99.632% <= 0.503 milliseconds (cumulative count 99632)
99.791% <= 0.607 milliseconds (cumulative count 99791)
99.856% <= 0.703 milliseconds (cumulative count 99856)
99.901% <= 0.807 milliseconds (cumulative count 99901)
99.926% <= 0.903 milliseconds (cumulative count 99926)
99.954% <= 1.007 milliseconds (cumulative count 99954)
99.986% <= 1.103 milliseconds (cumulative count 99986)
99.992% <= 1.207 milliseconds (cumulative count 99992)
99.995% <= 1.303 milliseconds (cumulative count 99995)
99.997% <= 1.503 milliseconds (cumulative count 99997)
99.999% <= 1.807 milliseconds (cumulative count 99999)
100.000% <= 3.103 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 106382.98 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        0.258     0.056     0.239     0.359     0.423     2.247
```















### redis-benchmark -p 6379 -t set,get -n 100000 -c 200 -d 1024
    - testing SET and GET 
    - 100000 times
    - 2000 concurrent parallel clients
    - 1-kilobyte each instead of the default of 3-bytes each

The highlights:
    - SET: ~81k ops/sec, p50 1.28ms, p99 1.63ms
    - GET: ~96k ops/sec, p50 1.03ms, p99 1.67ms
    - 99.9%+ of requests under 3ms
    - Throughput held within 10% of the 50-client run despite 4x concurrency and 340x payload size


```
WARNING: Could not fetch server CONFIG
====== SET ======                                                   
  100000 requests completed in 1.24 seconds
  200 parallel clients
  1024 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.103 milliseconds (cumulative count 1)
50.000% <= 1.279 milliseconds (cumulative count 50097)
75.000% <= 1.335 milliseconds (cumulative count 77537)
87.500% <= 1.367 milliseconds (cumulative count 88401)
93.750% <= 1.399 milliseconds (cumulative count 94275)
96.875% <= 1.439 milliseconds (cumulative count 97003)
98.438% <= 1.527 milliseconds (cumulative count 98451)
99.219% <= 1.687 milliseconds (cumulative count 99243)
99.609% <= 2.063 milliseconds (cumulative count 99615)
99.805% <= 2.599 milliseconds (cumulative count 99806)
99.902% <= 3.255 milliseconds (cumulative count 99903)
99.951% <= 4.215 milliseconds (cumulative count 99953)
99.976% <= 4.759 milliseconds (cumulative count 99977)
99.988% <= 4.927 milliseconds (cumulative count 99988)
99.994% <= 5.071 milliseconds (cumulative count 99994)
99.997% <= 5.135 milliseconds (cumulative count 99997)
99.998% <= 5.455 milliseconds (cumulative count 99999)
99.999% <= 5.623 milliseconds (cumulative count 100000)
100.000% <= 5.623 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
0.001% <= 0.103 milliseconds (cumulative count 1)
0.003% <= 0.207 milliseconds (cumulative count 3)
0.009% <= 0.407 milliseconds (cumulative count 9)
0.021% <= 0.503 milliseconds (cumulative count 21)
0.041% <= 0.607 milliseconds (cumulative count 41)
0.057% <= 0.703 milliseconds (cumulative count 57)
0.082% <= 0.807 milliseconds (cumulative count 82)
0.148% <= 0.903 milliseconds (cumulative count 148)
0.594% <= 1.007 milliseconds (cumulative count 594)
3.438% <= 1.103 milliseconds (cumulative count 3438)
19.698% <= 1.207 milliseconds (cumulative count 19698)
62.618% <= 1.303 milliseconds (cumulative count 62618)
95.106% <= 1.407 milliseconds (cumulative count 95106)
98.211% <= 1.503 milliseconds (cumulative count 98211)
98.909% <= 1.607 milliseconds (cumulative count 98909)
99.272% <= 1.703 milliseconds (cumulative count 99272)
99.425% <= 1.807 milliseconds (cumulative count 99425)
99.529% <= 1.903 milliseconds (cumulative count 99529)
99.584% <= 2.007 milliseconds (cumulative count 99584)
99.640% <= 2.103 milliseconds (cumulative count 99640)
99.878% <= 3.103 milliseconds (cumulative count 99878)
99.945% <= 4.103 milliseconds (cumulative count 99945)
99.996% <= 5.103 milliseconds (cumulative count 99996)
100.000% <= 6.103 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 80840.74 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        1.282     0.096     1.279     1.407     1.631     5.623
====== GET ======                                                    
  100000 requests completed in 1.04 seconds
  200 parallel clients
  1024 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.207 milliseconds (cumulative count 1)
50.000% <= 1.031 milliseconds (cumulative count 50828)
75.000% <= 1.231 milliseconds (cumulative count 75643)
87.500% <= 1.351 milliseconds (cumulative count 88200)
93.750% <= 1.423 milliseconds (cumulative count 93820)
96.875% <= 1.503 milliseconds (cumulative count 97056)
98.438% <= 1.599 milliseconds (cumulative count 98457)
99.219% <= 1.719 milliseconds (cumulative count 99252)
99.609% <= 1.855 milliseconds (cumulative count 99614)
99.805% <= 2.159 milliseconds (cumulative count 99805)
99.902% <= 2.471 milliseconds (cumulative count 99905)
99.951% <= 2.599 milliseconds (cumulative count 99952)
99.976% <= 2.951 milliseconds (cumulative count 99976)
99.988% <= 3.095 milliseconds (cumulative count 99991)
99.994% <= 3.119 milliseconds (cumulative count 99994)
99.997% <= 3.151 milliseconds (cumulative count 99997)
99.998% <= 3.175 milliseconds (cumulative count 99999)
99.999% <= 4.231 milliseconds (cumulative count 100000)
100.000% <= 4.231 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
0.000% <= 0.103 milliseconds (cumulative count 0)
0.001% <= 0.207 milliseconds (cumulative count 1)
0.003% <= 0.303 milliseconds (cumulative count 3)
0.020% <= 0.407 milliseconds (cumulative count 20)
0.035% <= 0.503 milliseconds (cumulative count 35)
0.068% <= 0.607 milliseconds (cumulative count 68)
0.861% <= 0.703 milliseconds (cumulative count 861)
9.953% <= 0.807 milliseconds (cumulative count 9953)
28.119% <= 0.903 milliseconds (cumulative count 28119)
47.581% <= 1.007 milliseconds (cumulative count 47581)
59.842% <= 1.103 milliseconds (cumulative count 59842)
72.938% <= 1.207 milliseconds (cumulative count 72938)
83.615% <= 1.303 milliseconds (cumulative count 83615)
92.767% <= 1.407 milliseconds (cumulative count 92767)
97.056% <= 1.503 milliseconds (cumulative count 97056)
98.523% <= 1.607 milliseconds (cumulative count 98523)
99.167% <= 1.703 milliseconds (cumulative count 99167)
99.534% <= 1.807 milliseconds (cumulative count 99534)
99.690% <= 1.903 milliseconds (cumulative count 99690)
99.757% <= 2.007 milliseconds (cumulative count 99757)
99.778% <= 2.103 milliseconds (cumulative count 99778)
99.993% <= 3.103 milliseconds (cumulative count 99993)
99.999% <= 4.103 milliseconds (cumulative count 99999)
100.000% <= 5.103 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 95969.28 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        1.068     0.200     1.031     1.447     1.671     4.231
```





### redis-benchmark -p 6379 -t set,get -n 100000 -c 200 -d 1024 -P 16
    - testing SET and GET 
    - 100000 times
    - 2000 concurrent parallel clients
    - 1-kilobyte each instead of the default of 3-bytes each
    - Pipeline mode ~ sends 16 commands per round-trip

The highlights:
    - SET: ~76k ops/sec, p50 0.38ms, p99 27ms
    - GET: ~78k ops/sec, p50 0.26ms, p99 6.97ms
    - Tail latency spiked hard — SET p99 jumped from 1.6ms to 27ms


```
WARNING: Could not fetch server CONFIG
====== SET ======                                                   
  100000 requests completed in 1.31 seconds
  200 parallel clients
  1024 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.055 milliseconds (cumulative count 128)
50.000% <= 0.383 milliseconds (cumulative count 51056)
75.000% <= 0.687 milliseconds (cumulative count 75216)
87.500% <= 1.255 milliseconds (cumulative count 87568)
93.750% <= 3.135 milliseconds (cumulative count 93760)
96.875% <= 16.927 milliseconds (cumulative count 96880)
98.438% <= 24.207 milliseconds (cumulative count 98448)
99.219% <= 28.623 milliseconds (cumulative count 99232)
99.609% <= 31.823 milliseconds (cumulative count 99616)
99.805% <= 33.343 milliseconds (cumulative count 99808)
99.902% <= 35.103 milliseconds (cumulative count 99904)
99.951% <= 36.159 milliseconds (cumulative count 99952)
99.976% <= 39.231 milliseconds (cumulative count 99984)
99.988% <= 41.087 milliseconds (cumulative count 100000)
100.000% <= 41.087 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
10.176% <= 0.103 milliseconds (cumulative count 10176)
23.776% <= 0.207 milliseconds (cumulative count 23776)
34.928% <= 0.303 milliseconds (cumulative count 34928)
54.832% <= 0.407 milliseconds (cumulative count 54832)
64.912% <= 0.503 milliseconds (cumulative count 64912)
71.856% <= 0.607 milliseconds (cumulative count 71856)
75.824% <= 0.703 milliseconds (cumulative count 75824)
78.816% <= 0.807 milliseconds (cumulative count 78816)
80.944% <= 0.903 milliseconds (cumulative count 80944)
82.864% <= 1.007 milliseconds (cumulative count 82864)
85.168% <= 1.103 milliseconds (cumulative count 85168)
86.912% <= 1.207 milliseconds (cumulative count 86912)
88.048% <= 1.303 milliseconds (cumulative count 88048)
89.072% <= 1.407 milliseconds (cumulative count 89072)
89.968% <= 1.503 milliseconds (cumulative count 89968)
90.656% <= 1.607 milliseconds (cumulative count 90656)
91.024% <= 1.703 milliseconds (cumulative count 91024)
91.584% <= 1.807 milliseconds (cumulative count 91584)
91.840% <= 1.903 milliseconds (cumulative count 91840)
92.000% <= 2.007 milliseconds (cumulative count 92000)
92.304% <= 2.103 milliseconds (cumulative count 92304)
93.744% <= 3.103 milliseconds (cumulative count 93744)
94.208% <= 4.103 milliseconds (cumulative count 94208)
94.384% <= 5.103 milliseconds (cumulative count 94384)
94.608% <= 6.103 milliseconds (cumulative count 94608)
94.880% <= 7.103 milliseconds (cumulative count 94880)
95.040% <= 8.103 milliseconds (cumulative count 95040)
95.184% <= 9.103 milliseconds (cumulative count 95184)
95.376% <= 10.103 milliseconds (cumulative count 95376)
95.536% <= 11.103 milliseconds (cumulative count 95536)
95.664% <= 12.103 milliseconds (cumulative count 95664)
95.792% <= 13.103 milliseconds (cumulative count 95792)
95.952% <= 14.103 milliseconds (cumulative count 95952)
96.288% <= 15.103 milliseconds (cumulative count 96288)
96.624% <= 16.103 milliseconds (cumulative count 96624)
96.928% <= 17.103 milliseconds (cumulative count 96928)
97.088% <= 18.111 milliseconds (cumulative count 97088)
97.328% <= 19.103 milliseconds (cumulative count 97328)
97.440% <= 20.111 milliseconds (cumulative count 97440)
97.616% <= 21.103 milliseconds (cumulative count 97616)
97.936% <= 22.111 milliseconds (cumulative count 97936)
98.240% <= 23.103 milliseconds (cumulative count 98240)
98.416% <= 24.111 milliseconds (cumulative count 98416)
98.672% <= 25.103 milliseconds (cumulative count 98672)
98.912% <= 26.111 milliseconds (cumulative count 98912)
99.008% <= 27.103 milliseconds (cumulative count 99008)
99.152% <= 28.111 milliseconds (cumulative count 99152)
99.264% <= 29.103 milliseconds (cumulative count 99264)
99.408% <= 30.111 milliseconds (cumulative count 99408)
99.504% <= 31.103 milliseconds (cumulative count 99504)
99.680% <= 32.111 milliseconds (cumulative count 99680)
99.776% <= 33.119 milliseconds (cumulative count 99776)
99.872% <= 34.111 milliseconds (cumulative count 99872)
99.904% <= 35.103 milliseconds (cumulative count 99904)
99.936% <= 36.127 milliseconds (cumulative count 99936)
99.952% <= 37.119 milliseconds (cumulative count 99952)
99.968% <= 38.111 milliseconds (cumulative count 99968)
99.984% <= 40.127 milliseconds (cumulative count 99984)
100.000% <= 41.119 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 76335.88 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        1.551     0.048     0.383     7.895    27.055    41.087
====== GET ======                                                   
  100000 requests completed in 1.28 seconds
  200 parallel clients
  1024 bytes payload
  keep alive: 1
  multi-thread: no

Latency by percentile distribution:
0.000% <= 0.039 milliseconds (cumulative count 32)
50.000% <= 0.263 milliseconds (cumulative count 51744)
75.000% <= 0.447 milliseconds (cumulative count 75296)
87.500% <= 0.767 milliseconds (cumulative count 87568)
93.750% <= 1.743 milliseconds (cumulative count 93776)
96.875% <= 4.175 milliseconds (cumulative count 96880)
98.438% <= 6.047 milliseconds (cumulative count 98448)
99.219% <= 7.375 milliseconds (cumulative count 99232)
99.609% <= 8.119 milliseconds (cumulative count 99616)
99.805% <= 8.391 milliseconds (cumulative count 99808)
99.902% <= 8.703 milliseconds (cumulative count 99936)
99.951% <= 8.799 milliseconds (cumulative count 99952)
99.976% <= 9.031 milliseconds (cumulative count 99984)
99.988% <= 9.143 milliseconds (cumulative count 100000)
100.000% <= 9.143 milliseconds (cumulative count 100000)

Cumulative distribution of latencies:
8.128% <= 0.103 milliseconds (cumulative count 8128)
37.232% <= 0.207 milliseconds (cumulative count 37232)
59.568% <= 0.303 milliseconds (cumulative count 59568)
72.368% <= 0.407 milliseconds (cumulative count 72368)
78.768% <= 0.503 milliseconds (cumulative count 78768)
83.424% <= 0.607 milliseconds (cumulative count 83424)
86.144% <= 0.703 milliseconds (cumulative count 86144)
88.432% <= 0.807 milliseconds (cumulative count 88432)
90.304% <= 0.903 milliseconds (cumulative count 90304)
91.968% <= 1.007 milliseconds (cumulative count 91968)
92.704% <= 1.103 milliseconds (cumulative count 92704)
93.024% <= 1.207 milliseconds (cumulative count 93024)
93.136% <= 1.303 milliseconds (cumulative count 93136)
93.520% <= 1.407 milliseconds (cumulative count 93520)
93.632% <= 1.503 milliseconds (cumulative count 93632)
93.696% <= 1.607 milliseconds (cumulative count 93696)
93.744% <= 1.703 milliseconds (cumulative count 93744)
93.776% <= 1.807 milliseconds (cumulative count 93776)
93.792% <= 1.903 milliseconds (cumulative count 93792)
93.920% <= 2.103 milliseconds (cumulative count 93920)
95.632% <= 3.103 milliseconds (cumulative count 95632)
96.784% <= 4.103 milliseconds (cumulative count 96784)
97.584% <= 5.103 milliseconds (cumulative count 97584)
98.464% <= 6.103 milliseconds (cumulative count 98464)
99.104% <= 7.103 milliseconds (cumulative count 99104)
99.584% <= 8.103 milliseconds (cumulative count 99584)
99.984% <= 9.103 milliseconds (cumulative count 99984)
100.000% <= 10.103 milliseconds (cumulative count 100000)

Summary:
  throughput summary: 77881.62 requests per second
  latency summary (msec):
          avg       min       p50       p95       p99       max
        0.586     0.032     0.263     2.631     6.975     9.143
```