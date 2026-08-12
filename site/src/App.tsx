import { DOWNLOADS } from "./download";

const startSteps = [
  {
    number: "01",
    title: "Baixe o launcher",
    text: "Um único download abre o caminho para sua próxima aventura.",
    image: "/images/voxtera-step-chest.png",
    alt: "Baú voxel do launcher",
  },
  {
    number: "02",
    title: "Instale o jogo",
    text: "O launcher cuida da instalação e das atualizações para você.",
    image: "/images/voxtera-step-portal.png",
    alt: "Portal voxel para instalar o jogo",
  },
  {
    number: "03",
    title: "Entre em Voxtera",
    text: "Escolha seu rumo e dê o primeiro passo quando estiver pronto.",
    image: "/images/voxtera-step-sword-shield.png",
    alt: "Espada e escudo voxel para entrar em Voxtera",
  },
];

function BuildIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="4"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m8 40 13-13" />
      <path d="m14 10 4-4 12 12-4 4" />
      <path d="m10 14 4-4 8 8-4 4" />
      <path d="m25 8 15 15" />
      <path d="m30 35 8 8" />
      <path d="m20 25 10 10" />
    </svg>
  );
}

function LauncherDownloads() {
  return (
    <div className="download-actions" aria-label="Downloads do launcher">
      <a className="button button-primary" href={DOWNLOADS.windows}>Baixar launcher para Windows (.exe)</a>
    </div>
  );
}

export function App() {
  return (
    <div className="site-shell">
      <main id="top">
        <section className="hero" aria-labelledby="hero-title">
          <img className="hero-image" src="/images/voxtera-clean-hero.png" alt="Vale ensolarado de Voxtera com aventureiro e vila" />
          <div className="hero-legibility" aria-hidden="true" />
          <header className="site-header page-width">
            <a className="brand" href="#top" aria-label="Voxtera — início">VOXTERA</a>
            <nav aria-label="Navegação principal">
              <a href="#game">O jogo</a>
              <a href="#start">Começar</a>
            </nav>
            <a className="header-download" href="#downloads">Baixar</a>
          </header>
          <div className="hero-content page-width">
            <h1 id="hero-title">Sua aventura começa aqui</h1>
            <p className="hero-copy">Voxtera é um mundo aberto para explorar, construir e transformar cada descoberta em uma história sua.</p>
            <LauncherDownloads />
            <p className="platform-note">Windows 10/11 · instalação e atualizações automáticas</p>
          </div>
          <a className="scroll-cue" href="#game">Conheça o mundo <span aria-hidden="true">↓</span></a>
        </section>

        <section id="game" className="game-intro section page-width" aria-labelledby="game-title">
          <h2 id="game-title">Um mundo feito para se perder de propósito</h2>
          <p>Todo vale esconde uma surpresa. Todo bloco pode ser o começo de algo maior.</p>
        </section>

        <section className="editorial-sections" aria-label="Formas de jogar">
          <article className="editorial editorial-explore" aria-labelledby="explore-title">
            <img src="/images/mountain-valley.jpg" alt="Vale montanhoso e arborizado em Voxtera" />
            <div className="editorial-shade" aria-hidden="true" />
            <div className="editorial-copy page-width">
              <p className="section-number">01</p>
              <h3 id="explore-title">Explore</h3>
              <p>Atravesse vales, picos gelados e florestas antigas em um mundo que convida você a seguir além do horizonte.</p>
            </div>
          </article>

          <article className="editorial editorial-build page-width" aria-labelledby="build-title">
            <div className="editorial-build-copy">
              <BuildIcon />
              <h3 id="build-title">Construa</h3>
              <div className="ornament" aria-hidden="true" />
              <p>Erga cidades, fortalezas e fazendas. Use blocos, recursos e sua criatividade para transformar o mundo do seu jeito.</p>
            </div>
            <img src="/images/voxtera-build-village.png" alt="Vila voxel ensolarada para construir em Voxtera" />
          </article>

          <article className="editorial editorial-adventure page-width" aria-labelledby="adventure-title">
            <div className="editorial-adventure-copy">
              <p className="section-number">02</p>
              <h3 id="adventure-title">Aventure-se</h3>
              <p>Descubra ruínas, encare criaturas e encontre novos caminhos onde a paisagem termina.</p>
            </div>
            <img src="/images/ruins-adventure.jpg" alt="Grupo em combate contra criaturas em ruínas vulcânicas de Voxtera" />
          </article>
        </section>

        <section id="start" className="start section page-width" aria-labelledby="start-title">
          <div className="start-heading">
            <h2 id="start-title">Como começar</h2>
            <p>Sem complicação: o launcher prepara o caminho para você entrar no jogo.</p>
          </div>
          <div className="steps-layout">
            <ol className="steps">
              {startSteps.map(({ number, title, text, image, alt }) => (
                <li key={number}>
                  <span className="step-number">{number}</span>
                  <img src={image} alt={alt} />
                  <h3>{title}</h3>
                  <p>{text}</p>
                </li>
              ))}
            </ol>
            <div className="steps-paths" aria-hidden="true">
              {startSteps.slice(0, -1).map(({ number }) => (
                <span className="steps-path" key={number} />
              ))}
            </div>
          </div>
        </section>

        <section id="downloads" className="download-band" aria-labelledby="download-title">
          <img src="/images/voxtera-closing-valley.png" alt="Vale voxel com aventureiro e lobo" />
          <div className="download-band-shade" aria-hidden="true" />
          <div className="page-width download-band-content">
            <h2 id="download-title">Pronto para começar sua aventura?</h2>
            <p>Baixe o launcher e encontre seu próprio caminho em Voxtera.</p>
            <LauncherDownloads />
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="page-width footer-content">
          <a className="footer-brand" href="#top">VOXTERA</a>
          <p>Um mundo aberto, bloco por bloco.</p>
          <a href="#downloads">Downloads</a>
        </div>
      </footer>
    </div>
  );
}
