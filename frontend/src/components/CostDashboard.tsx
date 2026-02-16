import { useEffect, useState } from "react";
import { getStats, listGoals, listAgents } from "../api/client";
import type { Stats, GoalSpace, AgentRun } from "../types";
import { DollarSign } from "lucide-react";
import { useToast } from "./ToastProvider";

interface GoalCost {
  goal: GoalSpace;
  cost: number;
}

export default function CostDashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [goalCosts, setGoalCosts] = useState<GoalCost[]>([]);
  const [agents, setAgents] = useState<AgentRun[]>([]);
  const { addToast } = useToast();

  useEffect(() => {
    getStats()
      .then(setStats)
      .catch(() => addToast("error", "Failed to load stats"));
    listAgents()
      .then(setAgents)
      .catch(() => addToast("error", "Failed to load agents"));
    listGoals()
      .then(async (goals) => {
        // Compute per-goal costs from agents
        const allAgents = await listAgents().catch(() => [] as AgentRun[]);
        const costs = goals.map((goal) => ({
          goal,
          cost: allAgents
            .filter((a) => a.goal_space_id === goal.id)
            .reduce((sum, a) => sum + a.cost_usd, 0),
        }));
        setGoalCosts(costs.sort((a, b) => b.cost - a.cost));
      })
      .catch(() => addToast("error", "Failed to load goal costs"));
  }, []);

  const maxGoalCost = Math.max(...goalCosts.map((g) => g.cost), 0.01);
  const maxAgentCost = Math.max(...agents.map((a) => a.cost_usd), 0.01);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-gray-100 mb-6">
          Cost Dashboard
        </h1>
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700 inline-block">
          <div className="flex items-center gap-3 text-gray-400 mb-2">
            <DollarSign size={20} />
            <span className="text-sm">Total Spend</span>
          </div>
          <span className="text-4xl font-bold font-mono text-green-400">
            ${(stats?.total_cost_usd ?? 0).toFixed(2)}
          </span>
        </div>
      </div>

      {/* Per-goal breakdown */}
      <div>
        <h2 className="text-lg font-semibold text-gray-100 mb-4">
          Cost by Goal
        </h2>
        <div className="space-y-3">
          {goalCosts.map(({ goal, cost }) => (
            <div key={goal.id} className="space-y-1">
              <div className="flex justify-between text-sm">
                <span className="text-gray-300 truncate">{goal.name}</span>
                <span className="text-gray-400 font-mono">
                  ${cost.toFixed(2)}
                </span>
              </div>
              <div className="h-3 bg-gray-800 rounded-full border border-gray-700 overflow-hidden">
                <div
                  className="h-full bg-blue-600 rounded-full transition-all"
                  style={{ width: `${(cost / maxGoalCost) * 100}%` }}
                />
              </div>
            </div>
          ))}
          {goalCosts.length === 0 && (
            <p className="text-gray-500 text-sm">No cost data available</p>
          )}
        </div>
      </div>

      {/* Per-agent cost */}
      <div>
        <h2 className="text-lg font-semibold text-gray-100 mb-4">
          Cost by Agent
        </h2>
        <div className="space-y-3">
          {agents
            .sort((a, b) => b.cost_usd - a.cost_usd)
            .map((agent) => (
              <div key={agent.id} className="space-y-1">
                <div className="flex justify-between text-sm">
                  <span className="text-gray-300 font-mono truncate">
                    {agent.branch ?? agent.id.slice(0, 8)}
                  </span>
                  <span className="text-gray-400 font-mono">
                    ${agent.cost_usd.toFixed(4)}
                  </span>
                </div>
                <div className="h-3 bg-gray-800 rounded-full border border-gray-700 overflow-hidden">
                  <div
                    className="h-full bg-green-600 rounded-full transition-all"
                    style={{
                      width: `${(agent.cost_usd / maxAgentCost) * 100}%`,
                    }}
                  />
                </div>
              </div>
            ))}
          {agents.length === 0 && (
            <p className="text-gray-500 text-sm">No agents recorded</p>
          )}
        </div>
      </div>
    </div>
  );
}
