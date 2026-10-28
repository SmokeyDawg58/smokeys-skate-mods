//! Shared EventSys queue envelope/EmbedField/Pop storage. EventSys.Init82985538,
//! EPostman.Create82983200, EmbedField829828d0, Queue.Pop829827b8.
use std::sync::Mutex;
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Entry<M>{pub(crate) event_size:usize,pub(crate) copied_message:Option<M>,pub(crate) message_published:bool}
#[derive(Debug,PartialEq,Eq)]
pub(crate) enum QueueError{Poisoned,Allocation,PendingOverflowDiagnostic,UnpublishedEvent,ConcurrentPop}
struct Fields<M>{creation:usize,deletion:usize,entries:Vec<Entry<M>>,current:bool}
pub(crate) struct Queue<M>{capacity:usize,fields:Mutex<Fields<M>>}
impl<M:Clone> Queue<M>{
    pub(crate) fn try_construct(capacity:usize)->Result<Self,QueueError>{
        let mut entries=Vec::new();entries.try_reserve_exact(capacity/16).map_err(|_|QueueError::Allocation)?;
        Ok(Self{capacity,fields:Mutex::new(Fields{creation:0,deletion:0,entries,current:false})})
    }
    pub(crate) fn enqueue(&self,message:M,message_size:usize)->Result<(),QueueError>{
        let mut fields=self.fields.lock().map_err(|_|QueueError::Poisoned)?;
        if fields.creation+16>=self.capacity{return Err(QueueError::PendingOverflowDiagnostic);}
        fields.creation+=16;
        fields.entries.push(Entry{event_size:16,copied_message:None,message_published:false});
        let allocation=message_size.checked_add(15).ok_or(QueueError::Allocation)?&!15;
        let entry=fields.entries.last_mut().expect("created envelope");
        entry.copied_message=Some(message);entry.event_size+=allocation;
        if fields.creation+allocation>=self.capacity{return Err(QueueError::PendingOverflowDiagnostic);}
        fields.creation+=allocation;
        fields.entries.last_mut().expect("same envelope").message_published=true;
        Ok(())
    }
    /// Delivery precedes deletion; source handlers can append during delivery.
    /// A checked host refusal retains the event and cursor, never drops it.
    pub(crate) fn pop<E>(&self,deliver:impl FnOnce(M)->Result<(),E>)->Result<bool,PopError<E>>{
        let (message,size)={
            let mut fields=self.fields.lock().map_err(|_|PopError::Queue(QueueError::Poisoned))?;
            if fields.current{return Err(PopError::Queue(QueueError::ConcurrentPop));}
            let Some(entry)=fields.entries.first() else{return Ok(false);};
            if !entry.message_published{return Err(PopError::Queue(QueueError::UnpublishedEvent));}
            let message=entry.copied_message.clone().ok_or(PopError::Queue(QueueError::UnpublishedEvent))?;
            let size=entry.event_size;fields.current=true;(message,size)
        };
        let result=deliver(message);
        let mut fields=self.fields.lock().map_err(|_|PopError::Queue(QueueError::Poisoned))?;
        fields.current=false;result.map_err(PopError::Delivery)?;
        fields.entries.remove(0);fields.deletion+=size;
        if fields.deletion==fields.creation{fields.deletion=0;fields.creation=0;}
        Ok(true)
    }
    #[cfg(test)]
    pub(crate) fn snapshot(&self)->Result<(usize,usize,Vec<Entry<M>>),QueueError>{
        let fields=self.fields.lock().map_err(|_|QueueError::Poisoned)?;
        Ok((fields.creation,fields.deletion,fields.entries.clone()))
    }
}
#[derive(Debug)]
pub(crate) enum PopError<E>{Queue(QueueError),Delivery(E)}
