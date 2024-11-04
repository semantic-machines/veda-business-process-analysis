import {Component, html, Backend} from 'veda-client';
import Translate from './Translate.js';

export default class Header extends Component(HTMLElement) {
  static tag = 'bpa-header';
  
  render() {
    return html`
      <header>
        <nav class="navbar navbar-expand-lg my-2">
          <div class="container px-3">
            <a class="navbar-brand position-relative me-2 p-0" href="./"><img src="images/optiflow.png" alt="logo" style="height:32px;"></a>
            <span class="navbar-brand me-3" about="v-bpa:BusinessProcessAnalysisApplication" property="rdfs:label"></span>
            <button class="navbar-toggler" type="button" data-bs-toggle="offcanvas" data-bs-target="#offcanvasNavbar" aria-controls="offcanvasNavbar">
              <span class="navbar-toggler-icon"></span>
            </button>
            <div class="offcanvas offcanvas-end" tabindex="-1" id="offcanvasNavbar" aria-labelledby="offcanvasNavbarLabel">
              <div class="offcanvas-header">
                <h5 class="offcanvas-title" id="offcanvasNavbarLabel">Меню</h5>
                <button type="button" class="btn-close" data-bs-dismiss="offcanvas" aria-label="Close"></button>
              </div>
              <div class="offcanvas-body justify-content-end align-items-center">
                <ul class="navbar-nav">
                  <!--li class="nav-item">
                    <button class="btn btn-outline-secondary me-3" is="${Translate}" data-lang="ru, en"></button>
                  </li-->
                  <li class="nav-item">
                    <span class="nav-link fs-5" about="${Backend.user}" property="rdfs:label"></span>
                  </li>
                  <li class="nav-item">
                    <button class="btn logout pt-2 bi bi-box-arrow-right fs-5"></button>
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </nav>
      </header>
    `;
  }
}
customElements.define(Header.tag, Header);
