import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  listGoals,
  createGoal,
  listAgents,
  nudgeAgent,
  killAgent,
  getStats,
  decomposeGoal,
  dispatchGoal,
} from "./client";

const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

function jsonResponse(data: unknown, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: () => Promise.resolve(data),
    text: () => Promise.resolve(JSON.stringify(data)),
  };
}

beforeEach(() => {
  mockFetch.mockReset();
});

describe("listGoals", () => {
  it("makes GET /api/goals and returns parsed JSON", async () => {
    const goals = [{ id: "1", name: "Goal 1" }];
    mockFetch.mockResolvedValueOnce(jsonResponse(goals));

    const result = await listGoals();

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/goals",
      expect.objectContaining({
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(goals);
  });
});

describe("createGoal", () => {
  it("makes POST with correct body", async () => {
    const newGoal = { name: "New", description: "Desc", repo_path: "/tmp" };
    const created = { id: "2", ...newGoal };
    mockFetch.mockResolvedValueOnce(jsonResponse(created));

    const result = await createGoal(newGoal);

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/goals",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(newGoal),
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(created);
  });
});

describe("listAgents", () => {
  it("makes GET /api/agents and returns parsed JSON", async () => {
    const agents = [{ id: "a1", status: "running" }];
    mockFetch.mockResolvedValueOnce(jsonResponse(agents));

    const result = await listAgents();

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/agents",
      expect.objectContaining({
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(agents);
  });
});

describe("nudgeAgent", () => {
  it("sends POST with message body", async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse(undefined));

    await nudgeAgent("agent-123", "wake up");

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/agents/agent-123/nudge",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ message: "wake up" }),
        headers: { "Content-Type": "application/json" },
      }),
    );
  });
});

describe("killAgent", () => {
  it("sends POST to correct endpoint", async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse(undefined));

    await killAgent("agent-456");

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/agents/agent-456/kill",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
  });
});

describe("getStats", () => {
  it("returns parsed stats", async () => {
    const stats = {
      active_agents: 3,
      total_cost_usd: 12.5,
      tasks_completed: 10,
      tasks_total: 20,
      goals_active: 2,
    };
    mockFetch.mockResolvedValueOnce(jsonResponse(stats));

    const result = await getStats();

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/stats",
      expect.objectContaining({
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(stats);
  });
});

describe("Error handling", () => {
  it("throws with status code when fetch returns non-ok status", async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse("Not Found", 404));

    await expect(listGoals()).rejects.toThrow("API 404");
  });

  it("throws with status code for 500", async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse("Server Error", 500));

    await expect(listAgents()).rejects.toThrow("API 500");
  });
});

describe("decomposeGoal", () => {
  it("sends POST to correct endpoint", async () => {
    const tasks = [{ id: "t1", title: "Task 1" }];
    mockFetch.mockResolvedValueOnce(jsonResponse(tasks));

    const result = await decomposeGoal("goal-1");

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/goals/goal-1/decompose",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(tasks);
  });
});

describe("dispatchGoal", () => {
  it("sends POST to correct endpoint", async () => {
    const agents = [{ id: "a1", status: "spawning" }];
    mockFetch.mockResolvedValueOnce(jsonResponse(agents));

    const result = await dispatchGoal("goal-2");

    expect(mockFetch).toHaveBeenCalledWith(
      "/api/goals/goal-2/dispatch",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
      }),
    );
    expect(result).toEqual(agents);
  });
});
