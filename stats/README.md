# Xenos stats

## Requirements:

- Authorize gmail usage
- Install requirements `pip install -r requirements.txt`
- Setup a filter in Gmail that tags every Screeps email with the "Screeps" tag
- __Docker__ and __Docker Compose__

## Runnig:

- `docker-compose up`
- Open `http://localhost:3000` in your browser
- Login using _admin admin_
- In a new shell: `python fetch_stats.py`

## Load default settings

- Select `Create > Import > Upload .json File`
- Upload `grafana-init.json`

## Sending Stats to your email

```js
function sendStats() {
    let cpu = Game.cpu.getUsed();
    let bucket = Game.cpu.bucket;
    let gcl = Game.gcl;
    let population = Object.keys(Game.creeps).length;
    let time = Game.time;
    let stats = {
        time,
        cpu,
        bucket,
        gcl,
        population
    };
    stats = JSON.stringify(stats);
    Game.notify(stats);
}
```
