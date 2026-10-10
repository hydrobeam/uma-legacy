"""Section 1, spark generation: the derived numbers. The g0 = 0.2 / 0.4 and x1.1 curve is an external community fit
on Champions Meeting room data, cited, not reproduced here."""
import common
from common import Claim

S = '1'


def claims():
    k = 1.1 ** 6
    return [
        Claim(S, '1.1^6', '1.771561', k),
        Claim(S, 'g white at n = 6', '0.3543', 0.2 * k),
        Claim(S, 'g gold at n = 6', '0.7086', 0.4 * k),
        Claim(S, 'curve fit: observations', '1.2M', status='external'),
        Claim(S, 'curve fit: p per tier', '0.84', status='external'),
        Claim(S, 'additive form rejected, p', '1e-14', status='external'),
    ]


if __name__ == '__main__': common.main(claims)
