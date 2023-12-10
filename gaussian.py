import math

def gauss(sigma, v):
    x = v[0]
    y = v[1]
    a = 1/(2 * math.pi * sigma**2)
    exp = -(x**2 + y**2)/(2 * sigma**2)

    res = a * math.e**exp

    return res

vals = [
    ("corner", [-2,-2]),
    ("outer_edge", [-2,-1]),
    ("outer_mid_edge", [-2, 0]),
    ("inner_corner", [-1, -1]),
    ("inner_edge", [-1, 0]),
    ("self", [0, 0])
]

sigma = 15

vals2 = []
for val in vals:
    v = val[0]
    g = gauss(sigma, val[1])
    print(f"{v} = {g}")
    vals2.append((v, g))

print()

suma = (vals2[0][1] * 4
+ vals2[1][1] * 8
+ vals2[2][1] * 4
+ vals2[3][1] * 4
+ vals2[4][1] * 4
+ vals2[5][1]
)

for val in vals2:
    v = val[0]
    g = val[1] / suma
    print(f"{v} = {g}")
