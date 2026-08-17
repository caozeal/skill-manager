import { Routes, Route } from "react-router-dom";
import { Sidebar } from "@/components/Sidebar";
import { SkillDetailDrawer } from "@/components/SkillDetailDrawer";
import { Dashboard } from "@/pages/Dashboard";
import { AgentPage } from "@/pages/AgentPage";
import { MatrixPage } from "@/pages/MatrixPage";

export default function App() {
  return (
    <div className="flex h-full">
      <Sidebar />
      <main className="flex-1 overflow-auto">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/agent/:id" element={<AgentPage />} />
          <Route path="/matrix" element={<MatrixPage />} />
        </Routes>
      </main>
      <SkillDetailDrawer />
    </div>
  );
}
