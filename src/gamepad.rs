use gilrs::{Event, EventType, GamepadId, Gilrs, ev::{self, state::GamepadState}};

pub struct Gamepad {
    pub gilrs: Gilrs,
    active_gamepad: Option<GamepadId>
}

impl Gamepad {

    pub fn new() -> Self {
        Self {
            gilrs: Gilrs::new().unwrap(),
            active_gamepad: None,
        }
    }

    pub fn update_active_gamepad(&mut self) {

        // always set the first gamepad to be the active gamepad
        for (gamepad_id, gamepad) in self.gilrs.gamepads() {
            //log::debug!("{:?}", gamepad_id);

            self.active_gamepad = Some(gamepad_id);
        }
    }
    
    pub fn get_events(&mut self) -> Vec<EventType> {

        let mut events = Vec::new();

        while let Some(event) = self.gilrs.next_event() {
            events.push(event.event);
        }

        events
        
    }
    pub fn is_pressed(&mut self, button: gilrs::Button) -> bool {

        self.active_gamepad
            .map_or(
                false, 
                |g| 
                self.gilrs.connected_gamepad(g)
                    .unwrap().is_pressed(button)
            )

        
    }

    pub fn state(&mut self) {
        // self.active_gamepad
        //     .map_or(
        //         None, 
        //         |g| 
        //         Some(
        //             self.gilrs.gamepad(g)
        //             .state()
        //         )
        //     )
    }


}