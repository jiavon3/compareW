import { useState } from "react";
import FolderComparePage from "./features/folder-compare/FolderComparePage";
import TextComparePage from "./features/text-compare/TextComparePage";

type Session = "text" | "folder";

export default function App() {
  const [session, setSession] = useState<Session>("text");
  const [drillIn, setDrillIn] = useState<{ left: string; right: string } | null>(
    null,
  );
  const [drillGen, setDrillGen] = useState(0);
  const [folderReset, setFolderReset] = useState(0);

  function openDrillIn(left: string, right: string) {
    setDrillIn({ left, right });
    setDrillGen((value) => value + 1);
  }

  function closeDrillIn() {
    setDrillIn(null);
  }

  function newFolderSession() {
    setDrillIn(null);
    setFolderReset((value) => value + 1);
  }

  return (
    <>
      <div className="session-view" hidden={session !== "text"}>
        <TextComparePage
          session="text"
          active={session === "text"}
          onSession={setSession}
        />
      </div>
      <div className="session-view" hidden={session !== "folder" || Boolean(drillIn)}>
        <FolderComparePage
          session="folder"
          onSession={setSession}
          onOpenText={openDrillIn}
          onClearDrillIn={closeDrillIn}
          resetToken={folderReset}
        />
      </div>
      {drillIn ? (
        <div className="session-view" hidden={session !== "folder"}>
          <TextComparePage
            key={drillGen}
            session="folder"
            active={session === "folder"}
            onSession={setSession}
            drillIn={drillIn}
            onBackToFolder={closeDrillIn}
            onNewSession={newFolderSession}
          />
        </div>
      ) : null}
    </>
  );
}
