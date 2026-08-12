import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";

afterEach(cleanup);

describe("App", () => {
  it("uses the approved hero adventure message", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Sua aventura começa aqui" }),
    ).toBeInTheDocument();
  });

  it("states only the verified Windows platform", () => {
    render(<App />);

    expect(
      screen.getByText("Windows 10/11 · instalação e atualizações automáticas"),
    ).toBeInTheDocument();
  });

  it("renders the approved cinematic sections without card or thumbnail composition", () => {
    render(<App />);

    expect(document.querySelector(".feature-card")).not.toBeInTheDocument();
    expect(document.querySelector(".step-illustration")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Explore" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Aventure-se" })).toBeInTheDocument();
  });

  it("exposes the verified Windows launcher download action", () => {
    render(<App />);

    const windowsDownloads = screen.getAllByRole("link", {
      name: "Baixar launcher para Windows (.exe)",
    });

    expect(windowsDownloads).toHaveLength(2);
    windowsDownloads.forEach((action) => {
      expect(action).toHaveAttribute("href", "/downloads/VoxteraLauncher-windows-v0.4.5.exe");
    });
    expect(screen.queryByRole("link", { name: "Baixar launcher para macOS (.app)" })).not.toBeInTheDocument();
  });

  it("renders the approved hero artwork with the verified Windows launcher download", () => {
    render(<App />);

    expect(screen.getByAltText("Vale ensolarado de Voxtera com aventureiro e vila")).toHaveAttribute(
      "src",
      "/images/voxtera-clean-hero.png",
    );
    expect(screen.getAllByRole("link", { name: "Baixar launcher para Windows (.exe)" })[0]).toHaveAttribute(
      "href",
      "/downloads/VoxteraLauncher-windows-v0.4.5.exe",
    );
  });

  it("does not expose GitHub links", () => {
    render(<App />);

    document.querySelectorAll<HTMLAnchorElement>("a").forEach((link) => {
      expect(link.href).not.toContain("github.com");
    });
  });

  it("renders the approved build and onboarding visual language", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Construa" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Como começar" })).toBeInTheDocument();
    expect(screen.getByAltText("Vila voxel ensolarada para construir em Voxtera")).toHaveAttribute("src", "/images/voxtera-build-village.png");
    expect(screen.getByAltText("Baú voxel do launcher")).toHaveAttribute("src", "/images/voxtera-step-chest.png");
    expect(screen.getByAltText("Portal voxel para instalar o jogo")).toHaveAttribute("src", "/images/voxtera-step-portal.png");
    expect(screen.getByAltText("Espada e escudo voxel para entrar em Voxtera")).toHaveAttribute("src", "/images/voxtera-step-sword-shield.png");
    expect(screen.getByAltText("Vale voxel com aventureiro e lobo")).toHaveAttribute("src", "/images/voxtera-closing-valley.png");
  });

  it("keeps the build article between exploration and adventure", () => {
    render(<App />);

    const editorialHeadings = Array.from(document.querySelectorAll(".editorial h3"), (heading) => heading.textContent);

    expect(editorialHeadings).toEqual(["Explore", "Construa", "Aventure-se"]);
  });

  it("renders only list items as direct children of the onboarding list", () => {
    render(<App />);

    const stepsList = document.querySelector("ol.steps");

    expect(stepsList).not.toBeNull();
    expect(Array.from(stepsList!.children)).toHaveLength(3);
    Array.from(stepsList!.children).forEach((child) => {
      expect(child.tagName).toBe("LI");
    });
  });

  it("renders the required onboarding titles", () => {
    render(<App />);

    ["Baixe o launcher", "Instale o jogo", "Entre em Voxtera"].forEach((title) => {
      expect(screen.getByRole("heading", { level: 3, name: title })).toBeInTheDocument();
    });
  });
});
