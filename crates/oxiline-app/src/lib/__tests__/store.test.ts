import { beforeEach, describe, expect, it } from "vitest";
import { useUi } from "../store";

describe("activity selection", () => {
  beforeEach(() => {
    useUi.setState({ selectedActivityIds: [] });
  });

  it("selects one activity and toggles additive selections", () => {
    const ui = useUi.getState();

    ui.toggleActivitySelect("activity-a", false);
    expect(useUi.getState().selectedActivityIds).toEqual(["activity-a"]);

    useUi.getState().toggleActivitySelect("activity-b", true);
    expect(useUi.getState().selectedActivityIds).toEqual(["activity-a", "activity-b"]);

    useUi.getState().toggleActivitySelect("activity-a", true);
    expect(useUi.getState().selectedActivityIds).toEqual(["activity-b"]);
  });

  it("clears the selected activities", () => {
    useUi.getState().toggleActivitySelect("activity-a", false);

    useUi.getState().clearActivitySelection();

    expect(useUi.getState().selectedActivityIds).toEqual([]);
  });
});
