import { useState } from "react";
import { Routes, Route } from "react-router-dom";
import FleetView from "@/components/FleetView";
import AgentDetail from "@/components/AgentDetail";
import GoalSpaceView from "@/components/GoalSpaceView";
import CostDashboard from "@/components/CostDashboard";
import { ToastProvider } from "@/components/ToastProvider";
import SidebarLayout from "@/components/layout/SidebarLayout";
import CommandPalette from "@/components/CommandPalette";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

function AppShell() {
  const [cmdOpen, setCmdOpen] = useState(false);

  useKeyboardShortcuts({
    onCommandPalette: () => setCmdOpen((v) => !v),
  });

  return (
    <>
      <SidebarLayout>
        <Routes>
          <Route path="/" element={<FleetView />} />
          <Route path="/agents/:id" element={<AgentDetail />} />
          <Route path="/goals" element={<GoalSpaceView />} />
          <Route path="/goals/:id" element={<GoalSpaceView />} />
          <Route path="/stats" element={<CostDashboard />} />
        </Routes>
      </SidebarLayout>
      <CommandPalette open={cmdOpen} onOpenChange={setCmdOpen} />
    </>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <AppShell />
    </ToastProvider>
  );
}
