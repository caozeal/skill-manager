import { cn } from "@/lib/utils";

export function TabBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "-mb-px border-b-2 px-3 py-2 text-[13px]",
        active
          ? "border-blue-500 font-medium text-gray-900"
          : "border-transparent text-gray-500 hover:text-gray-700",
      )}
    >
      {children}
    </button>
  );
}
