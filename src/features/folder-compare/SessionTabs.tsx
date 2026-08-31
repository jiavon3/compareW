import { IconExcel, IconFolder, IconText } from "../text-compare/icons";
import ToolButton from "../text-compare/ToolButton";

export type Session = "text" | "folder" | "excel";

type Props = {
  session: Session;
  onSession: (session: Session) => void;
};

export default function SessionTabs({ session, onSession }: Props) {
  return (
    <div className="scope session-tabs" role="tablist" aria-label="会话">
      <ToolButton
        kind="scope"
        icon={<IconText />}
        label="文本比对"
        pressed={session === "text"}
        onClick={() => onSession("text")}
      />
      <ToolButton
        kind="scope"
        icon={<IconFolder />}
        label="文件夹比对"
        pressed={session === "folder"}
        onClick={() => onSession("folder")}
      />
      <ToolButton
        kind="scope"
        icon={<IconExcel />}
        label="Excel比对"
        pressed={session === "excel"}
        onClick={() => onSession("excel")}
      />
    </div>
  );
}
