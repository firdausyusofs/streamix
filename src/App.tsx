import "./App.css";
import { Catalog } from "./pages/Catalog";
import { BrowserRouter, Route, Routes, useLocation } from "react-router";
import { MetaDetails } from "./pages/MetaDetails";
import { Explore } from "./pages/Explore";
import { Addons } from "./pages/Addons";
import { TabBar } from "./components/TabBar";

function Layout() {
  const location = useLocation();
  const hideTabBar = location.pathname.startsWith("/meta/");

  return (
    <div className="app-container">
      <main className={hideTabBar ? "" : "with-tab-bar"}>
        <Routes>
          <Route path="/" element={<Catalog />} />
          <Route path="/explore" element={<Explore />} />
          <Route path="/addons" element={<Addons />} />
          <Route path="/meta/:id" element={<MetaDetails />} />
        </Routes>
      </main>
      {!hideTabBar && <TabBar />}
    </div>
  );
}

function App() {
  return (
    <BrowserRouter>
      <Layout />
    </BrowserRouter>
  );
}

export default App;
