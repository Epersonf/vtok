def ship_cost(order):
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
