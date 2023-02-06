import logo from './dino.png';
import './App.css';

function App() {
  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Multiplayer Chrome Dino!
        </p>
        <a href="/games">Join the global games</a>
      </header>
    </div>
  );
}

export default App;
