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


{
  "symbols": [
    {
      "name": "graph",
      "kind": "variable",
      "identifier_position": {
        "path": "sample_project/python/main.py",
        "position": {
          "line": 5,
          "character": 0
        }
      },
      "range": {
        "path": "sample_project/python/main.py",
        "start": {
          "line": 5,
          "character": 0
        },
        "end": {
          "line": 5,
          "character": 20
        }
      }
    },
    {
      "name": "result",
      "kind": "variable",
      "identifier_position": {
        "path": "sample_project/python/main.py",
        "position": {
          "line": 6,
          "character": 0
        }
      },
      "range": {
        "path": "sample_project/python/main.py",
        "start": {
          "line": 6,
          "character": 0
        },
        "end": {
          "line": 6,
          "character": 12
        }
      }
    },
    {
      "name": "cost",
      "kind": "variable",
      "identifier_position": {
        "path": "sample_project/python/main.py",
        "position": {
          "line": 6,
          "character": 8
        }
      },
      "range": {
        "path": "sample_project/python/main.py",
        "start": {
          "line": 6,
          "character": 0
        },
        "end": {
          "line": 6,
          "character": 12
        }
      }
    }
  ],
  "referencing_symbols": [
    [],
    [],
    [],
    [
      {
        "name": "cost",
        "kind": "variable",
        "identifier_position": {
          "path": "sample_project/python/main.py",
          "position": {
            "line": 6,
            "character": 8
          }
        },
        "range": {
          "path": "sample_project/python/main.py",
          "start": {
            "line": 6,
            "character": 0
          },
          "end": {
            "line": 6,
            "character": 12
          }
        }
      }
    ],
    [],
    [],
    [],
    [
      {
        "name": "result",
        "kind": "variable",
        "identifier_position": {
          "path": "sample_project/python/main.py",
          "position": {
            "line": 6,
            "character": 0
          }
        },
        "range": {
          "path": "sample_project/python/main.py",
          "start": {
            "line": 6,
            "character": 0
          },
          "end": {
            "line": 6,
            "character": 12
          }
        }
      }
    ],
    []
  ],
  "referenced_symbols": [
    [
      {
        "name": "graph",
        "kind": "variable",
        "identifier_position": {
          "path": "sample_project/python/main.py",
          "position": {
            "line": 5,
            "character": 0
          }
        },
        "range": {
          "path": "sample_project/python/main.py",
          "start": {
            "line": 5,
            "character": 0
          },
          "end": {
            "line": 5,
            "character": 20
          }
        }
      },
      {
        "name": "AStarGraph",
        "kind": "class",
        "identifier_position": {
          "path": "sample_project/python/graph.py",
          "position": {
            "line": 1,
            "character": 6
          }
        },
        "range": {
          "path": "sample_project/python/graph.py",
          "start": {
            "line": 1,
            "character": 0
          },
          "end": {
            "line": 60,
            "character": 40
          }
        }
      }
    ],
    [],
    []
  ]
}