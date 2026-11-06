def ship_cost(order):
    if order is None:
        raise ValueError("no order")
    if order["qty"] <= 0:
        raise ValueError("bad qty")
    heavy = order["weight"] > 10
    cost = (20 if heavy else 10) if order["country"] == "US" else (45 if heavy else 30)
    if order.get("express"):
        cost = cost * 2
    return cost
