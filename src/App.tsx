import "./App.css";
import { Catalog } from "./pages/Catalog";
import { BrowserRouter, Route, Routes } from "react-router";
import { MetaDetails } from "./pages/MetaDetails";

function App() {
  return (
    <BrowserRouter>
      <div className="app-container">
        <main>
          <Routes>
            <Route path="/" element={<Catalog />} />
            <Route path="/meta/:id" element={<MetaDetails />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
