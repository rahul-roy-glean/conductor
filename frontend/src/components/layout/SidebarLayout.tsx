import { useState } from "react";
import { PanelLeftClose, PanelLeft } from "lucide-react";
import Sidebar from "@/components/layout/Sidebar";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export default function SidebarLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* Sidebar */}
      <aside
        className={cn(
          "border-r border-border bg-sidebar text-sidebar-foreground flex flex-col shrink-0 transition-all duration-200",
          collapsed ? "w-0 overflow-hidden" : "w-72",
        )}
      >
        <Sidebar />
      </aside>

      {/* Main content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Minimal top bar — collapse toggle only when sidebar hidden */}
        {collapsed && (
          <div className="px-3 py-2 border-b border-border">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => setCollapsed(false)}
              title="Show sidebar"
            >
              <PanelLeft size={16} />
            </Button>
          </div>
        )}

        {/* Scrollable content area */}
        <main className="flex-1 overflow-y-auto">
          <div className="max-w-7xl mx-auto px-6 py-6">{children}</div>
        </main>
      </div>

      {/* Collapse button overlaid at sidebar edge when open */}
      {!collapsed && (
        <Button
          variant="ghost"
          size="icon"
          className="fixed left-[268px] top-3 z-40 h-6 w-6 text-muted-foreground hover:text-foreground opacity-0 hover:opacity-100 transition-opacity"
          onClick={() => setCollapsed(true)}
          title="Hide sidebar"
        >
          <PanelLeftClose size={14} />
        </Button>
      )}
    </div>
  );
}
