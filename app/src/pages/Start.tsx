import { useLocation, useNavigate } from "react-router-dom";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";
import dryden from "../assets/ui/main-menu/dryden.png";
import mcdavid from "../assets/ui/main-menu/mcdavid.png";
import orr from "../assets/ui/main-menu/orr.png";

export function StartPage() {
  const navigate = useNavigate();
  const location = useLocation();
  return <main className="flex min-h-screen items-center justify-center bg-slate-950 p-6 text-slate-100">
    <div className="w-full max-w-5xl">
      <img src={headerLogo} alt="NHL Modding Studio" className="mx-auto mb-8 h-12" />
      <div className="grid gap-4 sm:grid-cols-3">
        {[
          { label: "Mod Manager", image: dryden, route: "/manager" },
          { label: "Modding Studio", image: mcdavid, route: "/studio" },
          { label: "Save Editor", image: orr, route: "/save-editor" },
        ].map(option => <button key={option.route} type="button"
          onClick={() => navigate(option.route, option.route === "/studio" ? { state: location.state } : undefined)}
          className="group relative isolate aspect-[420/747] w-full overflow-hidden rounded-xl border border-slate-700 bg-slate-900 text-left shadow-lg outline-none transition-colors hover:border-sky-400 focus-visible:border-sky-400 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:ring-offset-4 focus-visible:ring-offset-slate-950 sm:max-h-[calc(100svh-10rem)]">
          {/* Animate the image within its frame, leaving the label and layout stationary. */}
          <img src={option.image} alt="" draggable={false}
            className="absolute inset-0 h-full w-full object-cover grayscale transition-[filter,transform] duration-500 ease-out group-hover:scale-[1.04] group-hover:grayscale-0 group-focus-visible:scale-[1.04] group-focus-visible:grayscale-0 motion-reduce:transform-none motion-reduce:transition-none" />
          <span aria-hidden="true" className="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/0 to-transparent" />
          <span className="absolute inset-x-0 bottom-0 p-5 text-center text-xl font-semibold tracking-wide text-white sm:p-6 lg:text-2xl">{option.label}</span>
        </button>)}
      </div>
    </div>
  </main>;
}
