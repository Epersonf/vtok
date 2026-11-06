class Money:
    """Amount in minor units."""

    def __init__(self, cents, currency):
        self.cents = cents
        self.currency = currency

    def __eq__(self, other):
        return self.cents == other.cents and self.currency == other.currency

    def __repr__(self):
        return f"Money({self.cents}, {self.currency!r})"

    def format(self):
        return f"{self.cents // 100}.{self.cents % 100:02d} {self.currency}"


def zero(currency):
    return Money(0, currency)
