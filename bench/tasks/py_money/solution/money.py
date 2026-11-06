import re


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

    @staticmethod
    def parse(text):
        m = re.fullmatch(r"(\d+)(?:\.(\d{1,2}))? ([A-Za-z]{3})", text)
        if not m:
            raise ValueError("bad money")
        frac = (m.group(2) or "").ljust(2, "0")
        return Money(int(m.group(1)) * 100 + int(frac or 0), m.group(3).upper())

    def __add__(self, other):
        if self.currency != other.currency:
            raise ValueError("currency mismatch")
        return Money(self.cents + other.cents, self.currency)


def zero(currency):
    return Money(0, currency)
