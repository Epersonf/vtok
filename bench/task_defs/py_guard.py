"""Task `py_guard` (python, target orders.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="orders.py", cmd="py", instruction=
 "Refactor `ship_cost(order)` to use guard clauses / early returns (no `else` branches, at most one level of nesting) "
 "and fix this bug: an order with a negative or zero quantity must raise ValueError (right now it returns 0). Behaviour must otherwise stay identical.",
 src={"orders.py": '''def ship_cost(order):
    if order is not None:
        if order["qty"] > 0:
            if order["country"] == "US":
                if order["weight"] > 10:
                    cost = 20
                else:
                    cost = 10
            else:
                if order["weight"] > 10:
                    cost = 45
                else:
                    cost = 30
            if order.get("express"):
                cost = cost * 2
            return cost
        else:
            return 0
    else:
        raise ValueError("no order")
'''},
 tests={"test_orders.py": '''import ast, inspect, pytest
import orders
from orders import ship_cost

def o(**k):
    d = {"qty": 1, "country": "US", "weight": 5}; d.update(k); return d

def test_costs():
    assert ship_cost(o()) == 10
    assert ship_cost(o(weight=11)) == 20
    assert ship_cost(o(country="FR")) == 30
    assert ship_cost(o(country="FR", weight=11)) == 45
    assert ship_cost(o(express=True)) == 20

def test_errors():
    with pytest.raises(ValueError): ship_cost(None)
    with pytest.raises(ValueError): ship_cost(o(qty=0))
    with pytest.raises(ValueError): ship_cost(o(qty=-2))

def test_no_else():
    tree = ast.parse(inspect.getsource(orders))
    assert not any(isinstance(n, ast.If) and n.orelse for n in ast.walk(tree))
'''},
 solution={"orders.py": '''def ship_cost(order):
    if order is None:
        raise ValueError("no order")
    if order["qty"] <= 0:
        raise ValueError("bad qty")
    heavy = order["weight"] > 10
    cost = (20 if heavy else 10) if order["country"] == "US" else (45 if heavy else 30)
    if order.get("express"):
        cost = cost * 2
    return cost
'''})
