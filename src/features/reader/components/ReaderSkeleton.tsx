import { Skeleton } from "@/components/ui/skeleton";

export function ReaderSkeleton() {
  return (
    <div className="flex flex-col gap-5">
      <Skeleton className="h-8 w-2/3" />
      <Skeleton className="h-4 w-1/4" />
      {Array.from({ length: 8 }).map((_, index) => (
        <Skeleton key={index} className="h-20 w-full" />
      ))}
    </div>
  );
}

