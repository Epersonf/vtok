import ast, inspect, pytest
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
