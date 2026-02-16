import { Routes, Route, NavLink } from "react-router-dom";
import { Monitor, Target, BarChart3 } from "lucide-react";
import FleetView from "./components/FleetView";
import AgentDetail from "./components/AgentDetail";
import GoalSpaceView from "./components/GoalSpaceView";
import CostDashboard from "./components/CostDashboard";
import { ToastProvider } from "./components/ToastProvider";

function NavItem({
  to,
  icon: Icon,
  label,
}: {
  to: string;
  icon: typeof Monitor;
  label: string;
}) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `flex items-center gap-2 px-3 py-2 rounded text-sm font-medium transition-colors ${
          isActive
            ? "bg-gray-700/80 text-white"
            : "text-gray-400 hover:text-gray-200 hover:bg-gray-800/60"
        }`
      }
    >
      <Icon size={16} />
      {label}
    </NavLink>
  );
}

function Logo() {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="w-7 h-7"
    >
      <rect width="32" height="32" rx="8" fill="#1e293b" />
      <path
        d="M8 22 L24 8"
        stroke="#60a5fa"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
      <circle cx="24" cy="8" r="3" fill="#60a5fa" />
      <path
        d="M10 14 Q16 10 22 14"
        stroke="#34d399"
        strokeWidth="1.5"
        strokeLinecap="round"
        fill="none"
        opacity="0.7"
      />
      <path
        d="M10 18 Q16 14 22 18"
        stroke="#a78bfa"
        strokeWidth="1.5"
        strokeLinecap="round"
        fill="none"
        opacity="0.7"
      />
      <path
        d="M10 22 Q16 18 22 22"
        stroke="#fbbf24"
        strokeWidth="1.5"
        strokeLinecap="round"
        fill="none"
        opacity="0.7"
      />
    </svg>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <div className="min-h-screen bg-gray-900 text-gray-100">
        <nav className="border-b border-gray-800 px-6 py-3 flex items-center gap-6">
          <NavLink
            to="/"
            className="flex items-center gap-2.5 mr-4 hover:opacity-90 transition-opacity"
          >
            <Logo />
            <span className="text-lg font-bold tracking-tight text-white">
              Conductor
            </span>
          </NavLink>
          <NavItem to="/" icon={Monitor} label="Fleet" />
          <NavItem to="/goals" icon={Target} label="Goals" />
          <NavItem to="/stats" icon={BarChart3} label="Stats" />
          <a
            href="https://github.com/rahul-roy-glean/conductor"
            target="_blank"
            rel="noopener noreferrer"
            className="ml-auto text-gray-500 hover:text-gray-300 transition-colors"
            title="View on GitHub"
          >
            <svg
              viewBox="0 0 16 16"
              fill="currentColor"
              className="w-5 h-5"
              aria-hidden="true"
            >
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
            </svg>
          </a>
        </nav>
        <main className="max-w-7xl mx-auto px-6 py-6">
          <Routes>
            <Route path="/" element={<FleetView />} />
            <Route path="/agents/:id" element={<AgentDetail />} />
            <Route path="/goals" element={<GoalSpaceView />} />
            <Route path="/goals/:id" element={<GoalSpaceView />} />
            <Route path="/stats" element={<CostDashboard />} />
          </Routes>
        </main>
      </div>
    </ToastProvider>
  );
}
