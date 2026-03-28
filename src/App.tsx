import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { Catalog } from "./pages/Catalog";

function App() {
  return (
    <div className="app-container">
      <main>
        <Catalog />
      </main>
    </div>
  );
}

export default App;
