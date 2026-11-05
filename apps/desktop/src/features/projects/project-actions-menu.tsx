import { Archive, ArchiveRestore, FolderOpen, MoreHorizontal, Pencil, Star } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { ConfirmationDialog } from "@/components/confirmation-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { RenameProjectDialog } from "@/features/projects/rename-project-dialog";
import { useArchiveProject, useRestoreProject } from "@/features/projects/queries";
import type { Project } from "@/lib/domain";
import { getErrorMessage } from "@/lib/ipc";

export function ProjectActionsMenu({
  project,
  isCurrent,
  onSetCurrent,
}: {
  project: Project;
  isCurrent: boolean;
  onSetCurrent: (projectId: string) => void;
}) {
  const archiveProject = useArchiveProject();
  const restoreProject = useRestoreProject();
  const [renameOpen, setRenameOpen] = useState(false);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);

  const archived = Boolean(project.archivedAt);

  const confirmArchive = async () => {
    try {
      await archiveProject.mutateAsync(project.id);
      toast.success("项目已归档", {
        description: "磁盘文件与任务数据均保留，可随时恢复。",
      });
      setArchiveOpen(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  const confirmRestore = async () => {
    try {
      await restoreProject.mutateAsync(project.id);
      toast.success("项目已恢复");
      setRestoreOpen(false);
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`项目操作：${project.name}`}
          >
            <MoreHorizontal aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem asChild>
            <Link to={`/projects/${project.id}`}>
              <FolderOpen aria-hidden="true" />
              打开详情
            </Link>
          </DropdownMenuItem>
          {!archived ? (
            <DropdownMenuItem disabled={isCurrent} onSelect={() => onSetCurrent(project.id)}>
              <Star aria-hidden="true" />
              {isCurrent ? "当前项目" : "设为当前项目"}
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuItem onSelect={() => setRenameOpen(true)}>
            <Pencil aria-hidden="true" />
            重命名
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          {archived ? (
            <DropdownMenuItem onSelect={() => setRestoreOpen(true)}>
              <ArchiveRestore aria-hidden="true" />
              恢复项目
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem destructive onSelect={() => setArchiveOpen(true)}>
              <Archive aria-hidden="true" />
              归档项目
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      {renameOpen ? (
        <RenameProjectDialog open onOpenChange={setRenameOpen} project={project} />
      ) : null}

      <ConfirmationDialog
        open={archiveOpen}
        onOpenChange={setArchiveOpen}
        title="归档项目"
        description="项目会从活动列表隐藏，但不会删除磁盘文件、Git 仓库或任务数据。你可以在“已归档”中恢复。"
        confirmLabel="归档项目"
        destructive
        pending={archiveProject.isPending}
        onConfirm={() => void confirmArchive()}
      />
      <ConfirmationDialog
        open={restoreOpen}
        onOpenChange={setRestoreOpen}
        title="恢复项目"
        description="项目将重新出现在活动列表中，原有任务状态保持不变。"
        confirmLabel="恢复项目"
        pending={restoreProject.isPending}
        onConfirm={() => void confirmRestore()}
      />
    </>
  );
}
