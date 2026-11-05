function App() {
  return (
    <main className="grid min-h-screen place-items-center bg-slate-50 px-6 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <section className="w-full max-w-xl rounded-2xl border border-slate-200 bg-white p-10 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <p className="text-xs font-semibold tracking-[0.3em] text-sky-600 dark:text-sky-400">
          XUSHU
        </p>
        <h1 className="mt-4 text-3xl font-semibold tracking-tight">序枢 XUSHU</h1>
        <p className="mt-2 text-slate-600 dark:text-slate-400">本地优先的 AI 工作控制系统</p>
        <div className="mt-8 rounded-xl border border-dashed border-slate-300 p-5 text-sm dark:border-slate-700">
          <p className="font-medium">Phase 0 · 桌面工程骨架</p>
          <p className="mt-2 leading-6 text-slate-500 dark:text-slate-400">
            此页面仅用于验证 Tauri 2 + React + Vite 可以启动，不代表任何产品功能已完成。
          </p>
        </div>
      </section>
    </main>
  );
}

export default App;
