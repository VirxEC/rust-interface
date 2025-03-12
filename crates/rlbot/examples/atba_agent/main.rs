use std::{f32::consts::PI, num::NonZeroUsize, sync::Arc};

use rlbot::{
    RLBotConnection,
    agents::{Agent, PacketQueue, run_agents},
    flat::{ControllableInfo, ControllerState, FieldInfo, MatchConfiguration, PlayerInput},
    util::RLBotEnvironment,
};

#[allow(dead_code)]
struct AtbaAgent {
    index: u32,
    spawn_id: i32,
    team: u32,
    name: String,
    match_config: Arc<MatchConfiguration>,
    field_info: Arc<FieldInfo>,
}

impl Agent for AtbaAgent {
    fn new(
        team: u32,
        controllable_info: ControllableInfo,
        match_config: Arc<rlbot::flat::MatchConfiguration>,
        field_info: Arc<rlbot::flat::FieldInfo>,
        _packet_queue: &mut PacketQueue,
    ) -> Self {
        let name = match_config
            .player_configurations
            .iter()
            .find_map(|player| {
                if player.spawn_id == controllable_info.spawn_id {
                    Some(player.name.clone())
                } else {
                    None
                }
            })
            .unwrap();

        Self {
            index: controllable_info.index,
            spawn_id: controllable_info.spawn_id,
            team,
            name,
            match_config,
            field_info,
        }
    }

    fn tick(&mut self, game_packet: &rlbot::flat::GamePacket, packet_queue: &mut PacketQueue) {
        let Some(ball) = game_packet.balls.first() else {
            // If theres no ball, theres nothing to chase, don't do anything
            return;
        };

        // We're not in the gtp, skip this tick
        if game_packet.players.len() <= self.index as usize {
            return;
        }

        let target = &ball.physics;
        let car = game_packet.players[self.index as usize].physics;

        let bot_to_target_angle = f32::atan2(
            target.location.y - car.location.y,
            target.location.x - car.location.x,
        );

        let mut bot_front_to_target_angle = bot_to_target_angle - car.rotation.yaw;

        bot_front_to_target_angle = (bot_front_to_target_angle + PI).rem_euclid(2. * PI) - PI;

        let mut controller = ControllerState::default();

        if bot_front_to_target_angle > 0. {
            controller.steer = 1.;
        } else {
            controller.steer = -1.;
        }

        controller.throttle = 1.;

        packet_queue.push(PlayerInput {
            player_index: self.index,
            controller_state: controller,
        });
    }
}

fn main() {
    let RLBotEnvironment {
        server_addr,
        agent_id,
    } = RLBotEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "rlbot/rust-example/atba_agent".into());

    println!("Connecting");

    let rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    println!("Running!");

    // The hivemind field in your bot.toml file decides if rlbot core is going to
    // start your bot as one or multiple instances of your binary/exe.
    // If the hivemind field is set to true, one instance of your bot will handle
    // all of the bots in a team.

    // run N agents per thread
    // None runs 1 thread per agent
    let n_agents_per_thread = Some(NonZeroUsize::new(16).unwrap());

    // Blocking.
    run_agents::<AtbaAgent>(
        agent_id.clone(),
        true,
        true,
        rlbot_connection,
        n_agents_per_thread,
    )
    .expect("run_agents crashed");

    println!("Agent(s) with agent_id `{agent_id}` exited nicely");
}
