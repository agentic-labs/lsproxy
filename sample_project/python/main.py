import matplotlib.pyplot as plt
from graph import AStarGraph
from search import a_star_search


graph = AStarGraph()
result, cost = a_star_search((0, 0), (7, 7), graph)
print("route", result)
print("cost", cost)
plt.plot([v[0] for v in result], [v[1] for v in result])
for barrier in graph.barriers:
    plt.plot([v[0] for v in barrier], [v[1] for v in barrier])
plt.xlim(-1, 8)
plt.ylim(-1, 8)
plt.show()
