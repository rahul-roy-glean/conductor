import { Routes, Route, NavLink } from 'react-router-dom';
import { Monitor, Target, BarChart3 } from 'lucide-react';
import FleetView from './components/FleetView';
import AgentDetail from './components/AgentDetail';
import GoalSpaceView from './components/GoalSpaceView';
import CostDashboard from './components/CostDashboard';
import { ToastProvider } from './components/ToastProvider';

function NavItem({ to, icon: Icon, label }: { to: string; icon: typeof Monitor; label: string }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `flex items-center gap-2 px-3 py-2 rounded text-sm transition-colors ${
          isActive
            ? 'bg-gray-700 text-white'
            : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
        }`
      }
    >
      <Icon size={16} />
      {label}
    </NavLink>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <div className="min-h-screen bg-gray-900 text-gray-100">
        <nav className="border-b border-gray-800 px-6 py-3 flex items-center gap-6">
          <span className="text-lg font-bold tracking-tight text-white mr-4">Conductor</span>
          <NavItem to="/" icon={Monitor} label="Fleet" />
          <NavItem to="/goals" icon={Target} label="Goals" />
          <NavItem to="/stats" icon={BarChart3} label="Stats" />
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
