use std::time::Duration;
use xactor::*;

// This is a basic subscriber example to demonstrate usage of Sender
// We have actor A - SubscriberParent, manages a vec of child subscribers and in this example, sets up the message producer
// Actor B - (Child) Subscriber
// Actor C - Message Producer (Being subscribed to by the child subscribers) - producing a RandomMessage every few seconds to be broadcast to subscribers

#[xactor::main]
async fn main() -> std::io::Result<()> {
    let parent_addr = SubscriberParent::new().await.start().await.unwrap();
    parent_addr.wait_for_stop().await;
    Ok(())
}

// Subscriber Parent - A

struct SubscriberParent {
    children_subscribers: Vec<Addr<Subscriber>>,
    message_producer: Addr<MessageProducer>,
}

impl SubscriberParent {
    async fn new() -> SubscriberParent {
        SubscriberParent {
            children_subscribers: Vec::new(),
            message_producer: MessageProducer::new().start().await.unwrap(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for SubscriberParent {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        println!("Subscriber Parent Started");
        let _ = ctx.address().send(InitializeChildSubscribers);
        Ok(())
    }
}

#[message]
struct InitializeChildSubscribers;

#[async_trait::async_trait]
impl Handler<InitializeChildSubscribers> for SubscriberParent {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: InitializeChildSubscribers) {
        let message_producer_addr = self.message_producer.clone();
        let dummy_ids: Vec<i32> = vec![1, 2, 3, 4, 5];
        let children_unstarted_actors_vec = dummy_ids.into_iter().map(move |id| {
            let id = id.clone();
            let addr = message_producer_addr.clone();

            Subscriber::new(id, addr)
        });

        let children_addr_vec = children_unstarted_actors_vec
            .into_iter()
            .map(|actor| async { actor.start().await.unwrap() });

        let children_addr_vec = futures::future::join_all(children_addr_vec).await;

        self.children_subscribers = children_addr_vec;
    }
}

#[message]
struct ClearChildSubscribers;

#[async_trait::async_trait]
impl Handler<ClearChildSubscribers> for SubscriberParent {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: ClearChildSubscribers) {
        self.children_subscribers = Vec::new();
    }
}

// (Child) Subscriber - B

struct Subscriber {
    id: i32,
    message_producer_addr: Addr<MessageProducer>,
}

impl Subscriber {
    fn new(id: i32, message_producer_addr: Addr<MessageProducer>) -> Subscriber {
        Subscriber {
            id,
            message_producer_addr,
        }
    }
}

#[async_trait::async_trait]
impl Actor for Subscriber {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        // Send subscription request message to the Message Producer
        println!("Child Subscriber Started - id {:?}", self.id);
        let self_sender = ctx.address().sender();

        let _ = self.message_producer_addr.send(SubscribeToProducer {
            sender: self_sender.clone(),
        });

        let _ = self.message_producer_addr.send(SubscribeToProducer {
            sender: self_sender.clone(),
        });
        let _ = self.message_producer_addr.send(SubscribeToProducer {
            sender: self_sender.clone(),
        });
        let _ = self.message_producer_addr.send(SubscribeToProducer {
            sender: self_sender.clone(),
        });

        Ok(())
    }
}

#[async_trait::async_trait]
impl Handler<RandomMessage> for Subscriber {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: RandomMessage) {
        // We just print out the random id, along with the actor's unique id
        println!(
            "Child Subscriber (id: {:?}) Handling RandomMessage body: {:?}",
            self.id, msg.0
        );
    }
}

// Message Producer - C

struct MessageProducer {
    subscribers: Vec<Sender<RandomMessage>>,
}

impl MessageProducer {
    fn new() -> MessageProducer {
        MessageProducer {
            subscribers: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for MessageProducer {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        // Send broadcast message to self every 2 seconds
        println!("Message Producer Started");
        ctx.send_interval(Broadcast, Duration::from_secs(2));
        Ok(())
    }
}

#[message]
struct SubscribeToProducer {
    sender: Sender<RandomMessage>,
}

#[message]
#[derive(Clone)]
struct Broadcast;

#[message]
#[derive(Clone)]
struct RandomMessage(i32);

#[async_trait::async_trait]
impl Handler<Broadcast> for MessageProducer {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: Broadcast) {
        // Generate random number and broadcast that message to all subscribers
        println!("Broadcasting");
        // To avoid bringing in rand package for the sake of this example, we are hardcoding the "random" number
        let random_int: i32 = 20;
        let broadcast_message = RandomMessage(random_int);
        let _: Vec<_> = self
            .subscribers
            .iter()
            .map(|subscriber| subscriber.send(broadcast_message.clone()))
            .collect();
    }
}

#[async_trait::async_trait]
impl Handler<SubscribeToProducer> for MessageProducer {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: SubscribeToProducer) {
        println!("Recieved Subscription Request {:}", msg.sender.actor_id);
        self.subscribers.push(msg.sender);
    }
}
