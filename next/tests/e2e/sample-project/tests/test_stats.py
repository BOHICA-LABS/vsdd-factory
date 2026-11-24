"""Tests for tally.stats.

Every implemented behaviour has at least one test here. Behaviours that
are specified but not yet implemented (see README.md) deliberately have
none; the story that implements one adds its tests in the same run.
"""

import unittest

from tally import mean, variance


class TestMean(unittest.TestCase):
    def test_mean_of_integers(self):
        self.assertEqual(mean([1, 2, 3, 4]), 2.5)

    def test_mean_single_value(self):
        self.assertEqual(mean([5]), 5.0)

    def test_mean_is_order_independent(self):
        self.assertEqual(mean([3, 1, 2]), mean([1, 2, 3]))

    def test_mean_empty_raises(self):
        with self.assertRaises(ValueError):
            mean([])


class TestVariance(unittest.TestCase):
    def test_variance_is_zero_when_all_values_are_equal(self):
        self.assertEqual(variance([7, 7, 7]), 0.0)

    def test_population_variance_known_value(self):
        self.assertAlmostEqual(variance([2, 4, 4, 4, 5, 5, 7, 9]), 4.0)

    def test_variance_empty_raises(self):
        with self.assertRaises(ValueError):
            variance([])


if __name__ == "__main__":
    unittest.main()
