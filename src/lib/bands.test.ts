import { describe, expect, it } from "vitest";
import { groupBands } from "./bands";

describe("groupBands", () => {
  it("promedia grupos contiguos", () => {
    expect(groupBands([0, 10, 20, 30], 2)).toEqual([5, 25]);
  });

  it("repite bandas si se piden más barras que bandas", () => {
    expect(groupBands([10, 20], 4)).toEqual([10, 10, 20, 20]);
  });

  it("devuelve vacío sin datos o sin barras", () => {
    expect(groupBands([], 4)).toEqual([]);
    expect(groupBands([1, 2], 0)).toEqual([]);
  });
});
